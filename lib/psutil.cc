#include "include/psutil.h"

#include <chrono>
#include <thread>

#ifdef __APPLE__
#include <libproc.h>
#include <sys/proc_info.h>
#include <sys/sysctl.h>
#else
#include <cmath>
#include <cstdlib>
#include <fstream>
#include <sstream>
#include <unistd.h>
#include <vector>
#include <iostream>
#endif

int get_num_cores() {
#ifdef __APPLE__
  int nm[2];
  size_t len = 4;
  uint32_t count;

  nm[0] = CTL_HW;
  nm[1] = HW_AVAILCPU;
  sysctl(nm, 2, &count, &len, NULL, 0);

  if (count < 1) {
    nm[1] = HW_NCPU;
    sysctl(nm, 2, &count, &len, NULL, 0);
  }

  return count > 0 ? static_cast<int>(count) : 1;
#else
  return static_cast<int>(sysconf(_SC_NPROCESSORS_ONLN));
#endif
}

double get_cpu_time(int64_t pid) {
#ifdef __APPLE__
  struct proc_taskinfo pti;
  int ret = proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &pti, sizeof(pti));

  if (ret <= 0) {
    return 0.0;
  }

  return (pti.pti_total_user + pti.pti_total_system) / 1e9;
#else
  std::string stat_path = "/proc/" + std::to_string(pid) + "/stat";
  std::ifstream stat_file(stat_path);

  if (!stat_file.is_open()) {
    std::cerr << "Failed to open " << stat_path << std::endl;
    return -1.0;
  }

  std::string line;
  std::getline(stat_file, line);

  std::istringstream iss(line);
  std::string token;
  std::vector<std::string> tokens;

  while (std::getline(iss, token, ' ')) {
    tokens.push_back(token);
  }

  if (tokens.size() < 15) {
    std::cerr << "Unexpected format in " << stat_path << std::endl;
    return -1.0;
  }

  unsigned long long utime = std::stoull(tokens[13]);
  unsigned long long stime = std::stoull(tokens[14]);
  double ticks_per_second = static_cast<double>(sysconf(_SC_CLK_TCK));

  return (utime + stime) / ticks_per_second;
#endif
}

double get_process_cpu_usage_percentage(int64_t pid) {
  const std::chrono::milliseconds measurement_interval(100);
  double cpu_time_start = get_cpu_time(pid);

  if (cpu_time_start < 0) {
    return 0.0;
  }

  auto start_time = std::chrono::steady_clock::now();
  std::this_thread::sleep_for(measurement_interval);
  auto end_time = std::chrono::steady_clock::now();

  double cpu_time_end = get_cpu_time(pid);
  if (cpu_time_end < 0) {
    return 0.0;
  }

  long num_cores = get_num_cores();
  double cpu_time_diff = cpu_time_end - cpu_time_start;
  std::chrono::duration<double> elapsed = end_time - start_time;

  double elapsed_seconds = elapsed.count();
  double cpu_usage_percentage = (cpu_time_diff / elapsed_seconds) * (100.0 * num_cores);

  return std::min(cpu_usage_percentage, 100.0 * num_cores);
}