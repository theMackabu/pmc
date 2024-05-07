#ifndef BRIDGE_H
#define BRIDGE_H

#include <rust.h>
using namespace rust;

#ifndef CXXBRIDGE1_STRUCT_ProcessMetadata
#define CXXBRIDGE1_STRUCT_ProcessMetadata
struct ProcessMetadata final {
  String name;
  String shell;
  String command;
  String log_path;
  Vec<String> args;
  Vec<String> env;
  using IsRelocatable = std::true_type;
};
#endif

extern "C++" int64_t stop(int64_t pid);
extern "C++" int64_t run(ProcessMetadata metadata);
extern "C++" void set_program_name(String name);
extern "C++" int64_t get_child_pid(int64_t parentPID);
extern "C++" Vec<i64> find_chidren(int64_t pid);
#endif
