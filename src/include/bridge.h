#ifndef bridge
#define bridge

#include "rust.h"
using namespace rust;

int64_t stop(int64_t pid);
int64_t run(Str name, Str log_path, Str command);

#endif