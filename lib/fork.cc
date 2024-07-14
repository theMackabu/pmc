#include "include/fork.h"

#include <cstdlib>
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <unistd.h>

#ifdef _WIN32
#include <windows.h>
#else
#include <pwd.h>
#include <unistd.h>
#endif

using namespace std;

std::string home() {
#ifdef _WIN32
  const char *userProfile = std::getenv("USERPROFILE");
  if (userProfile) {
    return std::string(userProfile);
  } else {
    return "";
  }
#else
  struct passwd *pw = getpwuid(getuid());
  if (pw && pw->pw_dir) {
    return std::string(pw->pw_dir);
  } else {
    return "";
  }
#endif
}

Fork fork_process() {
  pid_t res = ::fork();
  if (res == -1) {
    perror("fork_process");
    throw std::runtime_error("fork() failed");
  } else if (res == 0) {
    return Fork::Child;
  } else {
    return Fork::Parent;
  }
}

pid_t set_sid() {
  pid_t res = ::setsid();
  if (res == -1) {
    perror("set_sid");
    throw std::runtime_error("setsid() failed");
  }
  return res;
}

void close_fd() {
  bool res = false;
  for (int i = 0; i <= 2; ++i) {
    res |= (::close(i) == -1);
  }

  if (res) {
    perror("close_fd");
    throw std::runtime_error("close_fd() failed");
  }
}

int32_t try_fork(bool nochdir, bool noclose, Callback callback) {
  try {
    Fork forkResult = fork_process();
    if (forkResult == Fork::Parent) {
      exit(0);
    } else if (forkResult == Fork::Child) {
      set_sid();
      if (!nochdir) {
        std::string home_dir = home() + ".pmc";
        chdir(home_dir.c_str());
      }
      if (!noclose) {
        close_fd();
      }
      forkResult = fork_process();
    }
    return static_cast<int32_t>(forkResult);
  } catch (const std::exception &e) {
    std::cerr << "[PMC] (cc) Error setting up daemon handler\n";
    perror("try_fork");
  }

  callback();
  return -1;
}