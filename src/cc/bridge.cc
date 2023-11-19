#include "../include/bridge.h"
#include "../include/process.h"

#include <signal.h>
#include <iostream>
#include <string>
using namespace std;

int64_t stop(int64_t pid) {
  return kill(pid, SIGTERM);
}

int64_t run(ProcessMetadata metadata) {  
  process::Runner runner;  
  runner.New(std::string(metadata.name), std::string(metadata.log_path));
  return runner.Run(std::string(metadata.command), std::string(metadata.shell), metadata.args);
}