#ifndef process
#define process

#include "rust.h"
using namespace rust;

namespace process {
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

#endif
