#include "include/process.h"

#include <algorithm>
#include <chrono>
#include <fcntl.h>
#include <fstream>
#include <iostream>
#include <signal.h>
#include <sstream>
#include <sys/wait.h>
#include <thread>
#include <unistd.h>

#ifdef __APPLE__
#include <sys/sysctl.h>
#include <sys/types.h>
#endif

using namespace std;

namespace process {
volatile sig_atomic_t childExitStatus = 0;

std::string format(std::string text) {
  std::replace(text.begin(), text.end(), ' ', '_');
  return text;
}

pair<std::string, std::string> split(const std::string &str) {
  size_t length = str.length();
  size_t midpoint = length / 2;

  std::string firstHalf = str.substr(0, midpoint);
  std::string secondHalf = str.substr(midpoint);

  return make_pair(firstHalf, secondHalf);
}

void sigchld_handler(int signo) {
  (void)signo;
  int status;
  while (waitpid(-1, &status, WNOHANG) > 0) {
    childExitStatus = status;
  }
}

void Runner::New(const std::string &name, const std::string &logPath) {
  std::string formattedName = format(name);
  std::string stdoutFileName = logPath + "/" + formattedName + "-out.log";
  std::string stderrFileName = logPath + "/" + formattedName + "-error.log";

  stdout_fd = open(stdoutFileName.c_str(), O_WRONLY | O_CREAT | O_APPEND, 0644);
  stderr_fd = open(stderrFileName.c_str(), O_WRONLY | O_CREAT | O_APPEND, 0644);

  struct sigaction sa;
  sa.sa_handler = sigchld_handler;
  sigemptyset(&sa.sa_mask);
  sa.sa_flags = SA_RESTART | SA_NOCLDSTOP;
  if (sigaction(SIGCHLD, &sa, NULL) == -1) {
    std::cerr << "[PMC] (cc) Error setting up SIGCHLD handler\n";
    perror("Runner::New");
  }
}

Runner::~Runner() {
  if (stdout_fd != -1) {
    close(stdout_fd);
  }

  if (stderr_fd != -1) {
    close(stderr_fd);
  }
}

int64_t Runner::Run(const std::string &command, const std::string &shell, Vec<String> args, Vec<String> env) {
  pid_t pid = fork();

  if (pid == -1) {
    std::cerr << "[PMC] (cc) Unable to fork\n";
    perror("Runner::Run");
    return -1;
  } else if (pid == 0) {
    setsid();

    close(STDIN_FILENO);
    close(STDOUT_FILENO);
    close(STDERR_FILENO);

    dup2(stdout_fd, STDOUT_FILENO);
    dup2(stderr_fd, STDERR_FILENO);

    std::vector<const char *> argsArray;
    std::vector<const char *> envArray;
    argsArray.push_back(shell.c_str());

    transform(args.begin(), args.end(), std::back_inserter(argsArray),
      [](rust::String& arg) { return arg.c_str(); });
      
    transform(env.begin(), env.end(), std::back_inserter(envArray),
      [](rust::String& env) { return env.c_str(); });

    argsArray.push_back(command.c_str());
    argsArray.push_back(nullptr);
    envArray.push_back(nullptr);

    if (execve(shell.c_str(), const_cast<char* const*>(argsArray.data()), const_cast<char* const*>(envArray.data())) == -1) {
      std::cerr << "[PMC] (cc) Unable to execute the command\n";
      perror("execvp");
      exit(EXIT_FAILURE);
    }
  } else {
    close(stdout_fd);
    close(stderr_fd);

    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    std::string proc_path = "/proc/" + std::to_string(pid) + "/task/" + std::to_string(pid) + "/children";
    
    std::ifstream proc_file(proc_path);
    if (proc_file.is_open()) {
      std::string line;
      if (std::getline(proc_file, line)) {
        std::istringstream iss(line);
        pid_t child_pid;
        if (iss >> child_pid) {
          return child_pid;
        }
      }
    }

    return pid;
  }

  return -1;
}} 
