#include "include/cmd.h"
#include <fcntl.h>
#include <unistd.h>

namespace cmd {
void Runner::New(const std::string &name, const std::string &logPath) {
  std::string stdoutFileName = logPath + "/" + name + "-stdout.log";
  std::string stderrFileName = logPath + "/" + name + "-stderr.log";
  
  stdout_fd = open(stdoutFileName.c_str(), O_WRONLY | O_CREAT | O_APPEND, 0644);
  stderr_fd = open(stderrFileName.c_str(), O_WRONLY | O_CREAT | O_APPEND, 0644);
}

Runner::~Runner() {
  if (stdout_fd != -1) {
    close(stdout_fd);
  }

  if (stderr_fd != -1) {
    close(stderr_fd);
  }
}

uint64_t Runner::Run(const std::string &command) {
  pid_t pid = fork();

  if (pid == -1) {
    std::cerr << "Error: Unable to fork.\n";
    return -1;
  } else if (pid == 0) {
    setsid();

    close(STDIN_FILENO);
    close(STDOUT_FILENO);
    close(STDERR_FILENO);

    dup2(stdout_fd, STDOUT_FILENO);
    dup2(stderr_fd, STDERR_FILENO);

    if (execl("/bin/bash", "bash", "-c", command.c_str(), (char *)nullptr) == -1) {
      std::cerr << "Error: Unable to execute the command.\n";
      exit(EXIT_FAILURE);
    }
  } else {
    return pid;
  }
  
  close(stdout_fd);
  close(stderr_fd);

  return -1;
}}

uint64_t run_command(Str name, Str log_path, Str command) {
  cmd::Runner runner;
  runner.New(std::string(name), std::string(log_path));
  return runner.Run(std::string(command));
}
