#include "../include/process.h"
#include <fcntl.h>
#include <unistd.h>
#include <sys/wait.h>
#include <signal.h>
#include <iostream>

namespace process {
volatile sig_atomic_t childExitStatus = 0;

void sigchld_handler(int signo) {
  (void)signo;
  int status;
  while (waitpid(-1, &status, WNOHANG) > 0) {
    childExitStatus = status;
  }
}

void Runner::New(const std::string &name, const std::string &logPath) {
  std::string stdoutFileName = logPath + "/" + name + "-out.log";
  std::string stderrFileName = logPath + "/" + name + "-error.log";
  
  stdout_fd = open(stdoutFileName.c_str(), O_WRONLY | O_CREAT | O_APPEND, 0644);
  stderr_fd = open(stderrFileName.c_str(), O_WRONLY | O_CREAT | O_APPEND, 0644);

  struct sigaction sa;
  sa.sa_handler = sigchld_handler;
  sigemptyset(&sa.sa_mask);
  sa.sa_flags = SA_RESTART | SA_NOCLDSTOP;
  if (sigaction(SIGCHLD, &sa, NULL) == -1) {
    std::cerr << "[PMC] (cc) Error setting up SIGCHLD handler\n";
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

int64_t Runner::Run(const std::string &command) {
  pid_t pid = fork();

  if (pid == -1) {
    std::cerr << "[PMC] (cc) Unable to fork\n";
    return -1;
  } else if (pid == 0) {
    setsid();

    close(STDIN_FILENO);
    close(STDOUT_FILENO);
    close(STDERR_FILENO);

    dup2(stdout_fd, STDOUT_FILENO);
    dup2(stderr_fd, STDERR_FILENO);

    if (execl("/bin/bash", "bash", "-c", command.c_str(), (char *)nullptr) == -1) {
      std::cerr << "[PMC] (cc) Unable to execute the command\n";
      exit(EXIT_FAILURE);
    }
  } else {
    close(stdout_fd);
    close(stderr_fd);

    return pid;
  }
  
  return -1;
}} 
