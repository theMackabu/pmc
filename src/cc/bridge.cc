#include "../include/bridge.h"
#include "process.cc"
#include <signal.h>

uint64_t stop(uint64_t pid) {
  return kill(pid, SIGTERM);
}

uint64_t run(Str name, Str log_path, Str command) {
  process::Runner runner;
  runner.New(std::string(name), std::string(log_path));
  return runner.Run(std::string(command));
}