#include "../include/bridge.h"
#include "../include/process.h"

#include <signal.h>
#include <stdio.h>
#include <iostream>
#include <string>
#include <sys/wait.h>
#include <sys/types.h>
using namespace std;

int64_t stop(int64_t pid) {
  vector<pid_t> children;
  pid_t child;

  while ((child = waitpid(-1, NULL, WNOHANG)) > 0) {
    children.push_back(child);
  }

  for (size_t i = 0; i < children.size(); i++) {
    kill(children[i], SIGTERM);
  }

  return kill(pid, SIGTERM);
}


int64_t run(ProcessMetadata metadata) {  
  process::Runner runner;  
  runner.New(std::string(metadata.name), std::string(metadata.log_path));
  return runner.Run(std::string(metadata.command), std::string(metadata.shell), metadata.args);
}