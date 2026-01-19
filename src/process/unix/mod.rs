use std::thread;
use std::time::{Duration, SystemTime};

pub mod cpu;
pub mod env;
pub mod memory;
pub mod process_info;
pub mod process_list;

pub use cpu::get_cpu_percent;
pub use env::{Vars, env};
pub use memory::{NativeMemoryInfo, get_memory_info};
pub use process_info::{get_parent_pid, get_process_name, get_process_start_time};
pub use process_list::native_processes;

pub const PROCESS_OPERATION_DELAY_MS: u64 = 100;

#[derive(Debug, Clone)]
pub struct NativeProcess {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
    pub memory_info: Option<NativeMemoryInfo>,
    pub cpu_percent: f64,
    pub create_time: SystemTime,
}

impl NativeProcess {
    pub fn new(pid: u32) -> Result<Self, String> {
        let ppid = get_parent_pid(pid as i32)?.map(|p| p as u32);
        let name = get_process_name(pid)?;
        let memory_info = get_memory_info(pid).ok();
        let cpu_percent = get_cpu_percent(pid);
        let create_time = get_process_start_time(pid)?;

        Ok(NativeProcess {
            pid,
            ppid,
            name,
            memory_info,
            cpu_percent,
            create_time,
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn ppid(&self) -> Result<Option<u32>, String> {
        Ok(self.ppid)
    }
    pub fn name(&self) -> Result<String, String> {
        Ok(self.name.clone())
    }
    pub fn memory_info(&self) -> Result<NativeMemoryInfo, String> {
        self.memory_info
            .clone()
            .ok_or_else(|| "Memory info not available".to_string())
    }
    pub fn cpu_percent(&self) -> Result<f64, String> {
        Ok(self.cpu_percent)
    }
}

#[cfg(target_os = "linux")]
pub fn get_actual_child_pid(shell_pid: i64) -> i64 {
    thread::sleep(Duration::from_millis(PROCESS_OPERATION_DELAY_MS));

    let proc_path = format!("/proc/{}/task/{}/children", shell_pid, shell_pid);
    if let Ok(contents) = std::fs::read_to_string(&proc_path)
        && let Some(child_pid_str) = contents.split_whitespace().next()
        && let Ok(child_pid) = child_pid_str.parse::<i64>()
    {
        return child_pid;
    }

    if let Ok(processes) = native_processes() {
        processes
            .iter()
            .find(|process| {
                if let Ok(Some(ppid)) = process.ppid() {
                    ppid as i64 == shell_pid
                } else {
                    false
                }
            })
            .map_or(shell_pid, |p| p.pid() as i64)
    } else {
        shell_pid
    }
}

#[cfg(target_os = "macos")]
pub fn get_actual_child_pid(shell_pid: i64) -> i64 {
    // Wait for shell to spawn the actual command
    thread::sleep(Duration::from_millis(PROCESS_OPERATION_DELAY_MS));

    // Find children by iterating processes
    if let Ok(processes) = native_processes() {
        for process in processes {
            let ppid = match process.ppid() {
                Ok(Some(ppid)) => ppid as i64,
                _ => continue,
            };

            if ppid == shell_pid {
                return process.pid() as i64;
            }
        }
    }

    // Fallback: try using sysctl or other macOS specific methods
    for test_pid in 1..32768 {
        if let Ok(Some(ppid)) = get_parent_pid(test_pid)
            && ppid as i64 == shell_pid
        {
            return test_pid as i64;
        }
    }

    shell_pid
}
