#include <iostream>
#include <string>
#include <fstream>
#include <sstream>
#include <vector>
#include <cstdlib>
#include <unistd.h>
#include <chrono>
#include <thread>
#include <cmath>
#ifdef __APPLE__
#include <libproc.h>
#include <sys/proc_info.h>
#endif

double get_process_cpu_usage_percentage(int64_t pid) {
	 auto get_cpu_time = [](int64_t pid) -> double {
#ifdef __APPLE__
		  struct proc_taskinfo pti;
		  int ret = proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &pti, sizeof(pti));
		  if (ret <= 0) {
				std::cerr << "Error: Unable to get process info" << std::endl;
				return -1.0;
		  }
		  return (pti.pti_total_user + pti.pti_total_system) / 100000000.0; // Convert nanoseconds to seconds
#else
		  std::string stat_path = "/proc/" + std::to_string(pid) + "/stat";
		  std::ifstream stat_file(stat_path);
		  
		  if (!stat_file.is_open()) {
				std::cerr << "Error: Unable to open " << stat_path << std::endl;
				return -1.0;
		  }
		  
		  std::string line;
		  std::getline(stat_file, line);
		  stat_file.close();
		  
		  std::istringstream iss(line);
		  std::string token;
		  std::vector<std::string> tokens;
		  
		  while (std::getline(iss, token, ' ')) {
				tokens.push_back(token);
		  }
		  
		  if (tokens.size() < 15) {
				std::cerr << "Error: Invalid stat file format" << std::endl;
				return -1.0;
		  }
		  
		  unsigned long long utime = std::stoull(tokens[13]);
		  unsigned long long stime = std::stoull(tokens[14]);
		  
		  return (utime + stime) / sysconf(_SC_CLK_TCK);
#endif
	 };

	 double cpu_time1 = get_cpu_time(pid);
	 if (cpu_time1 < 0) return -1.0;

	 auto start = std::chrono::high_resolution_clock::now();
	 std::this_thread::sleep_for(std::chrono::milliseconds(1000));
	 auto end = std::chrono::high_resolution_clock::now();

	 double cpu_time2 = get_cpu_time(pid);
	 if (cpu_time2 < 0) return -1.0;

	 std::chrono::duration<double> elapsed = end - start;
	 double elapsed_seconds = elapsed.count();
	 double cpu_time_diff = cpu_time2 - cpu_time1;

	 long num_cores = sysconf(_SC_NPROCESSORS_ONLN);
	 double cpu_usage_percentage = (cpu_time_diff / elapsed_seconds) * 100.0 * num_cores;

	 return std::min(cpu_usage_percentage, 100.0 * num_cores);
}
