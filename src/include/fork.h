#ifndef FORK_H
#define FORK_H
#include <string>

#ifndef CXXBRIDGE1_ENUM_Fork
#define CXXBRIDGE1_ENUM_Fork
enum class Fork: std::uint8_t {
    Parent,
    Child
};
#endif

using Callback = void(*)();
pid_t set_sid();
void close_fd();
Fork fork_process();
extern "C" int32_t try_fork(bool nochdir, bool noclose, Callback callback);
#endif
