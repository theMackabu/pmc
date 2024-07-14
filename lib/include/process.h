#ifndef PROCESS_H
#define PROCESS_H

#include "rust.h"
using namespace rust;

namespace process {
  class Runner {
  public:
    void New(const std::string &name, const std::string &logPath);
    int64_t Run(const std::string &command, const std::string &shell, Vec<String> args, Vec<String> env);
    ~Runner();

  private:
    int stdout_fd;
    int stderr_fd;
  };
}

#endif
