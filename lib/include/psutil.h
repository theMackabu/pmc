#ifndef PSUTIL_H
#define PSUTIL_H

#include <rust.h>
using namespace rust;

extern "C++" double get_process_cpu_usage_percentage(int64_t pid);
#endif
