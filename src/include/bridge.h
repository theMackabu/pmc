#ifndef bridge
#define bridge

#include "rust.h"
using namespace rust;

uint64_t stop(uint64_t pid);
uint64_t run(Str name, Str log_path, Str command);

#endif