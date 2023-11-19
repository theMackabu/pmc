#ifndef FORK_H
#define FORK_H
#include <string>

#ifndef CXXBRIDGE1_ENUM_Fork
#define CXXBRIDGE1_ENUM_Fork
enum class Fork: uint8_t {
    Parent,
    Child
};
#endif

using Callback = void(*)();
extern "C" pid_t set_sid();
extern "C" void close_fd();
extern "C" Fork fork_process();
extern "C" int chdir(const char* dir);
extern "C" int32_t try_fork(bool nochdir, bool noclose, Callback callback);
#endif
