#ifndef cmd
#define cmd

#include "cxx.h"
#include <fstream>
#include <string>
#include <iostream>
#include <cstring>
#include <cstdlib>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
using namespace rust;

namespace cmd {
  class Runner {
  public:
    void New(const std::string &name, const std::string &logPath);
    uint64_t Run(const std::string &command);
    ~Runner();

  private:
    int stdout_fd;
    int stderr_fd;
  };
}

uint64_t run_command(Str name, Str log_path, Str command);
uint64_t kill_pid(uint64_t pid);

#endif
