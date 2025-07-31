use std::time::SystemTime;

pub mod cpu;
pub mod env;
pub mod memory;
pub mod process_info;
pub mod process_list;

pub use cpu::get_cpu_percent;
pub use env::{env, Vars};
pub use memory::{get_memory_info, NativeMemoryInfo};
pub use process_info::{get_parent_pid, get_process_name, get_process_start_time};
pub use process_list::native_processes;

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
    
    pub fn pid(&self) -> u32 { self.pid }
    pub fn ppid(&self) -> Result<Option<u32>, String> { Ok(self.ppid) }
    pub fn name(&self) -> Result<String, String> { Ok(self.name.clone()) }
    pub fn memory_info(&self) -> Result<NativeMemoryInfo, String> {
        self.memory_info.clone().ok_or_else(|| "Memory info not available".to_string())
    }
    pub fn cpu_percent(&self) -> Result<f64, String> { Ok(self.cpu_percent) }
} 