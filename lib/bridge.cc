#include <bridge.h>
#include <process.h>
#include <iostream>
#include <signal.h>
#include <stdio.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <vector>
#ifdef __linux__
#include <cstring>
#include <string>
#include <cstdlib>
#include <dirent.h>
#include <sys/prctl.h>
#elif __APPLE__
#include <libproc.h>
#include <sys/proc_info.h>
#include <libproc.h>
#include <sys/proc_info.h>
#include <iostream>
#include <crt_externs.h>
#endif

using namespace std;

void set_program_name(String name) {
  #ifdef __linux__
  prctl(PR_SET_NAME, name.c_str());
  #elif __APPLE__
  setprogname(name.c_str());
  #endif
}

int64_t get_child_pid(int64_t parentPID) {
#ifdef __linux__
  DIR *dir = opendir("/proc");
  if (!dir) {
    std::cerr << "[PMC] (cc) Error opening /proc directory.\n";
    perror("get_child_pid");
    return -1;
  }

  int targetPID = -1;
  dirent *entry;

  while ((entry = readdir(dir)) != nullptr) {
    if (entry->d_type == DT_DIR && isdigit(entry->d_name[0])) {
      int pid = atoi(entry->d_name);
      char statusPath[256];
      snprintf(statusPath, sizeof(statusPath), "/proc/%d/status", pid);

      FILE *statusFile = fopen(statusPath, "r");
      if (statusFile) {
        char buffer[256];
        while (fgets(buffer, sizeof(buffer), statusFile) != nullptr) {
          if (strncmp(buffer, "PPid:", 5) == 0) {
            int parentID;
            if (sscanf(buffer + 5, "%d", &parentID) == 1 && parentID == parentPID) {
              targetPID = pid; break;
            } break;
          }
        }
        fclose(statusFile);
      }
    }
  }

  closedir(dir);
  return targetPID;
#elif __APPLE__
  pid_t pidList[1024];
  int count = proc_listpids(PROC_ALL_PIDS, 0, pidList, sizeof(pidList));

  if (count <= 0) {
    std::cerr << "Error retrieving process list." << std::endl;
    perror("get_child_pid");
    return -1;
  }

  for (int i = 0; i < count; ++i) {
    struct proc_bsdinfo procInfo;
    if (proc_pidinfo(pidList[i], PROC_PIDTBSDINFO, 0, &procInfo, sizeof(procInfo)) > 0) {
      if (procInfo.pbi_ppid == parentPID) {
        return static_cast<int>(pidList[i]);
      }
    }
  }

  return -1;
#else
  return -1;
#endif
}

rust::Vec<rust::i64> find_chidren(int64_t pid) {
  rust::Vec<rust::i64> children;
  int64_t child;

  while ((child = get_child_pid(pid)) != -1) {
    children.push_back(child);
    pid = child;
  }

  return children;
}

int64_t stop(int64_t pid) {
  vector<pid_t> children;
  int64_t child;

  while ((child = get_child_pid(pid)) != -1) {
    children.push_back(child);
    pid = child;
  }

  for (size_t i = 0; i < children.size(); i++) {
    kill(children[i], SIGTERM);
  }

  return kill(pid, SIGTERM);
}

int64_t run(ProcessMetadata metadata) {
  process::Runner runner;
  runner.New(std::string(metadata.name), std::string(metadata.log_path));
  return runner.Run(std::string(metadata.command), std::string(metadata.shell), metadata.args, metadata.env);
}