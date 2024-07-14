#include "include/psutil.h"

#include <chrono>
#include <thread>
#include <unordered_map>

#ifdef __APPLE__
#include <mach/mach.h>
#include <libproc.h>
#include <sys/proc_info.h>
#include <sys/sysctl.h>
#else
#include <fstream>
#include <cmath>
#include <sstream>
#include <vector>
#include <unistd.h>
#include <iostream>
#endif

struct CPUTime {
#ifdef __APPLE__
  uint64_t user;
  uint64_t system;
#else
  unsigned long long utime;
  unsigned long long stime;
  unsigned long long cutime;
  unsigned long long cstime;
#endif
};

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

CPUTime get_cpu_time(int64_t pid) {
#ifdef __APPLE__
  struct proc_taskinfo pti;
  int ret = proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &pti, sizeof(pti));
    
  if (ret <= 0) {
    return {0, 0};
  }
    
  return {pti.pti_total_user, pti.pti_total_system};
#else
  std::string stat_path = "/proc/" + std::to_string(pid) + "/stat";
  std::ifstream stat_file(stat_path);
  CPUTime result = {0, 0, 0, 0};

  if (!stat_file.is_open()) {
    std::cerr << "Failed to open " << stat_path << std::endl;
    return result;
  }

  std::string line;
  std::getline(stat_file, line);

  std::istringstream iss(line);
  std::string token;
  std::vector<std::string> tokens;

  while (std::getline(iss, token, ' ')) {
    tokens.push_back(token);
  }

  if (tokens.size() < 17) {
    std::cerr << "Unexpected format in " << stat_path << std::endl;
    return result;
  }

  result.utime = std::stoull(tokens[13]);
  result.stime = std::stoull(tokens[14]);
  result.cutime = std::stoull(tokens[15]);
  result.cstime = std::stoull(tokens[16]);

  return result;
#endif
}

double get_process_cpu_usage_percentage(int64_t pid) {
  static std::unordered_map<int64_t, CPUTime> last_cpu_times;
  static std::unordered_map<int64_t, double> last_cpu_percentages;
  const std::chrono::milliseconds measurement_interval(200);
  static int num_cores = get_num_cores();

  CPUTime start_time = get_cpu_time(pid);
  auto start = std::chrono::steady_clock::now();

  std::this_thread::sleep_for(measurement_interval);

  CPUTime end_time = get_cpu_time(pid);
  auto end = std::chrono::steady_clock::now();

  double elapsed_seconds = std::chrono::duration<double>(end - start).count();

  if (last_cpu_times.find(pid) == last_cpu_times.end()) {
    last_cpu_times[pid] = start_time;
    last_cpu_percentages[pid] = 0.0;
    return 0.0;
  }

  CPUTime& last_time = last_cpu_times[pid];

#ifdef __APPLE__
  uint64_t user_ticks = end_time.user - last_time.user;
  uint64_t system_ticks = end_time.system - last_time.system;
  uint64_t total_ticks = user_ticks + system_ticks;
  double seconds = static_cast<double>(total_ticks) / 1e9;
#else
  unsigned long long total_time = 
    (end_time.utime + end_time.stime + end_time.cutime + end_time.cstime) - 
    (last_time.utime + last_time.stime + last_time.cutime + last_time.cstime);
  
  double seconds = static_cast<double>(total_time) / sysconf(_SC_CLK_TCK);
#endif
  double cpu_usage = 100.0 * (seconds / elapsed_seconds) / num_cores;
  last_cpu_times[pid] = end_time;

  double& last_percentage = last_cpu_percentages[pid];
  cpu_usage = (cpu_usage * 0.3) + (last_percentage * 0.2);
  last_percentage = cpu_usage;

  return std::min(cpu_usage, 100.0 * num_cores);
}
