use std::ffi::{CStr, OsString};
use std::os::unix::prelude::OsStringExt;
use std::time::SystemTime;

pub struct Vars {
    inner: std::vec::IntoIter<OsString>,
}

impl Iterator for Vars {
    type Item = String;
    fn next(&mut self) -> Option<String> { self.inner.next().map(|var| var.into_string().unwrap()) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}

/// Native memory info structure to replace psutil's MemoryInfo
#[derive(Debug, Clone)]
pub struct NativeMemoryInfo {
    pub rss: u64,  // Resident Set Size
    pub vms: u64,  // Virtual Memory Size
}

impl NativeMemoryInfo {
    pub fn rss(&self) -> u64 { self.rss }
    pub fn vms(&self) -> u64 { self.vms }
}

/// Native process structure to replace psutil's Process
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

/// Get all running processes
pub fn native_processes() -> Result<Vec<NativeProcess>, String> {
    #[cfg(target_os = "macos")]
    {
        use std::mem;
        
        // Define macOS kinfo_proc structure (simplified)
        #[repr(C)]
        struct KinfoProc {
            kp_proc: ExternProc,
            // We only need the process part for our purposes
        }
        
        #[repr(C)]
        struct ExternProc {
            p_un: [u8; 16],           // Union, simplified as byte array
            p_vmspace: u64,
            p_sigacts: u64,
            p_flag: libc::c_int,
            p_stat: libc::c_char,
            p_pid: libc::pid_t,
            p_oppid: libc::pid_t,
            p_dupfd: libc::c_int,
            // ... other fields we don't need, represented as padding
            _padding: [u8; 400],      // Simplified padding to match struct size
        }
        
        let mut name: [i32; 3] = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_ALL];
        let mut size: libc::size_t = 0;
        
        // Get required buffer size
        let result = unsafe {
            libc::sysctl(
                name.as_mut_ptr(),
                name.len() as u32,
                std::ptr::null_mut(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        
        if result < 0 {
            return Err("Failed to get process list size".to_string());
        }
        
        // Allocate buffer
        let num_processes = size / mem::size_of::<KinfoProc>();
        let mut processes_buf: Vec<KinfoProc> = Vec::with_capacity(num_processes);
        
        let result = unsafe {
            libc::sysctl(
                name.as_mut_ptr(),
                name.len() as u32,
                processes_buf.as_mut_ptr() as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        
        if result < 0 {
            return Err("Failed to get process list".to_string());
        }
        
        unsafe {
            processes_buf.set_len(size / mem::size_of::<KinfoProc>());
        }
        
        let mut result_processes = Vec::new();
        
        for kinfo in processes_buf {
            let pid = kinfo.kp_proc.p_pid as u32;
            if let Ok(process) = NativeProcess::new(pid) {
                result_processes.push(process);
            }
        }
        
        Ok(result_processes)
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        let mut processes = Vec::new();
        
        // Read /proc directory
        let proc_dir = fs::read_dir("/proc")
            .map_err(|e| format!("Failed to read /proc: {}", e))?;
        
        for entry in proc_dir {
            if let Ok(entry) = entry &&
                let Ok(file_name) = entry.file_name().into_string() &&
                let Ok(pid) = file_name.parse::<u32>() &&
                let Ok(process) = NativeProcess::new(pid) {

                processes.push(process);
            }
        }
        
        Ok(processes)
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported platform".to_string())
    }
}

fn get_process_name(pid: u32) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        use std::mem;
        
        const PROC_PIDTBSDINFO: i32 = 3;
        
        #[repr(C)]
        struct ProcBsdInfo {
            pbi_flags: u32,
            pbi_status: u32,
            pbi_xstatus: u32,
            pbi_pid: u32,
            pbi_ppid: u32,
            pbi_uid: u32,
            pbi_gid: u32,
            pbi_ruid: u32,
            pbi_rgid: u32,
            pbi_svuid: u32,
            pbi_svgid: u32,
            rfu_1: u32,
            pbi_comm: [libc::c_char; 16],
            pbi_name: [libc::c_char; 32],
            // ... rest of fields
            _padding: [u8; 200], // Simplified padding
        }
        
        unsafe extern "C" {
            fn proc_pidinfo(
                pid: libc::c_int,
                flavor: libc::c_int,
                arg: u64,
                buffer: *mut libc::c_void,
                buffersize: libc::c_int,
            ) -> libc::c_int;
        }
        
        let mut proc_info: ProcBsdInfo = unsafe { mem::zeroed() };
        let result = unsafe {
            proc_pidinfo(
                pid as i32,
                PROC_PIDTBSDINFO,
                0,
                &mut proc_info as *mut _ as *mut libc::c_void,
                mem::size_of::<ProcBsdInfo>() as i32,
            )
        };
        
        if result <= 0 {
            return Err(format!("Failed to get process name for PID {}", pid));
        }
        
        let name = unsafe { CStr::from_ptr(proc_info.pbi_comm.as_ptr()) };
        Ok(name.to_string_lossy().into_owned())
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        let comm_path = format!("/proc/{}/comm", pid);
        fs::read_to_string(&comm_path)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("Failed to read process name: {}", e))
    }
}

fn get_memory_info(pid: u32) -> Result<NativeMemoryInfo, String> {
    #[cfg(target_os = "macos")]
    {
        use std::mem;
        
        const PROC_PIDTASKINFO: i32 = 4;
        
        #[repr(C)]
        struct ProcTaskInfo {
            pti_virtual_size: u64,
            pti_resident_size: u64,
            // ... other fields we don't need
            _padding: [u8; 200],
        }
        
        unsafe extern "C" {
            fn proc_pidinfo(
                pid: libc::c_int,
                flavor: libc::c_int,
                arg: u64,
                buffer: *mut libc::c_void,
                buffersize: libc::c_int,
            ) -> libc::c_int;
        }
        
        let mut task_info: ProcTaskInfo = unsafe { mem::zeroed() };
        let result = unsafe {
            proc_pidinfo(
                pid as i32,
                PROC_PIDTASKINFO,
                0,
                &mut task_info as *mut _ as *mut libc::c_void,
                mem::size_of::<ProcTaskInfo>() as i32,
            )
        };
        
        if result <= 0 {
            return Err(format!("Failed to get memory info for PID {}", pid));
        }
        
        Ok(NativeMemoryInfo {
            rss: task_info.pti_resident_size,
            vms: task_info.pti_virtual_size,
        })
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        let status_path = format!("/proc/{}/status", pid);
        let status_content = fs::read_to_string(&status_path)
            .map_err(|e| format!("Failed to read process status: {}", e))?;
        
        let mut rss = 0;
        let mut vms = 0;
        
        for line in status_content.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    rss = value.parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                }
            } else if line.starts_with("VmSize:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    vms = value.parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                }
            }
        }
        
        Ok(NativeMemoryInfo { rss, vms })
    }
}

fn get_cpu_percent(_pid: u32) -> f64 {
    // Simplified CPU calculation - would need more sophisticated implementation
    // for accurate percentage over time
    0.0
}

fn get_process_start_time(_pid: u32) -> Result<SystemTime, String> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::time::Duration;
        
        let stat_path = format!("/proc/{}/stat", _pid);
        let stat_content = fs::read_to_string(&stat_path)
            .map_err(|e| format!("Failed to read process stat: {}", e))?;
        
        let parts: Vec<&str> = stat_content.split_whitespace().collect();
        if parts.len() > 21 {
            if let Ok(start_time) = parts[21].parse::<u64>() {
                // Convert from clock ticks to seconds (simplified)
                let uptime_secs = start_time / 100;
                return Ok(UNIX_EPOCH + Duration::from_secs(uptime_secs));
            }
        }
    }
    
    // Fallback to current time for macOS and other systems
    Ok(SystemTime::now())
}

#[cfg(target_os = "macos")]
unsafe fn environ() -> *mut *const *const libc::c_char {
    let environ = unsafe { libc::_NSGetEnviron() };
    environ as *mut *const *const libc::c_char
}

#[cfg(not(target_os = "macos"))]
unsafe fn environ() -> *mut *const *const libc::c_char {
    unsafe extern "C" {
        static mut environ: *const *const libc::c_char;
    }
    std::ptr::addr_of_mut!(environ)
}

pub fn env() -> Vec<String> {
    unsafe {
        let mut environ = *environ();
        let mut result = Vec::new();

        if !environ.is_null() {
            while !(*environ).is_null() {
                if let Some(key_value) = parse(CStr::from_ptr(*environ).to_bytes()) {
                    result.push(key_value);
                }
                environ = environ.add(1);
            }
        }

        return Vars { inner: result.into_iter() }.collect();
    }

    fn parse(input: &[u8]) -> Option<OsString> {
        if input.is_empty() {
            return None;
        }
        Some(OsString::from_vec(input.to_vec()))
    }
}

/// Get parent process ID for a given process ID on macOS
#[cfg(target_os = "macos")]
pub fn get_parent_pid(pid: i32) -> Result<Option<i32>, String> {
    use std::mem;
    
    // macOS specific constants and structures
    const PROC_PIDTBSDINFO: i32 = 3;
    
    #[repr(C)]
    struct ProcBsdInfo {
        pbi_flags: u32,
        pbi_status: u32,
        pbi_xstatus: u32,
        pbi_pid: u32,
        pbi_ppid: u32,
        pbi_uid: u32,
        pbi_gid: u32,
        pbi_ruid: u32,
        pbi_rgid: u32,
        pbi_svuid: u32,
        pbi_svgid: u32,
        rfu_1: u32,
        pbi_comm: [libc::c_char; 16],
        pbi_name: [libc::c_char; 32],
        pbi_nfiles: u32,
        pbi_pgid: u32,
        pbi_pjobc: u32,
        e_tdev: u32,
        e_tpgid: u32,
        pbi_nice: i32,
        pbi_start_tvsec: u64,
        pbi_start_tvusec: u64,
    }
    
    unsafe extern "C" {
        fn proc_pidinfo(
            pid: libc::c_int,
            flavor: libc::c_int,
            arg: u64,
            buffer: *mut libc::c_void,
            buffersize: libc::c_int,
        ) -> libc::c_int;
    }
    
    let mut proc_info: ProcBsdInfo = unsafe { mem::zeroed() };
    let result = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDTBSDINFO,
            0,
            &mut proc_info as *mut _ as *mut libc::c_void,
            mem::size_of::<ProcBsdInfo>() as i32,
        )
    };
    
    if result <= 0 {
        return Err(format!("Failed to get process info for PID {}", pid));
    }
    
    let ppid = proc_info.pbi_ppid as i32;
    if ppid == 0 {
        Ok(None) // No parent (e.g., init process)
    } else {
        Ok(Some(ppid))
    }
}

/// Get parent process ID for Linux and other Unix systems
#[cfg(not(target_os = "macos"))]
pub fn get_parent_pid(pid: i32) -> Result<Option<i32>, String> {
    use std::fs;
    
    let stat_path = format!("/proc/{}/stat", pid);
    let stat_content = fs::read_to_string(&stat_path)
        .map_err(|e| format!("Failed to read {}: {}", stat_path, e))?;
    
    // Parse /proc/pid/stat format
    // The format is: pid (comm) state ppid ...
    let parts: Vec<&str> = stat_content.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(format!("Invalid stat format for PID {}", pid));
    }
    
    let ppid = parts[3].parse::<i32>()
        .map_err(|e| format!("Failed to parse ppid: {}", e))?;
    
    if ppid == 0 {
        Ok(None)
    } else {
        Ok(Some(ppid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_env_parsing() {
        let env_vars = env();
        assert!(!env_vars.is_empty());
        
        // Check if some common environment variables exist
        let has_path = env_vars.iter().any(|var| var.starts_with("PATH="));
        assert!(has_path, "PATH environment variable should exist");
    }
    
    #[test]
    fn test_get_parent_pid_current_process() {
        let current_pid = std::process::id() as i32;
        
        match get_parent_pid(current_pid) {
            Ok(Some(ppid)) => {
                assert!(ppid > 0, "Parent PID should be positive");
                assert_ne!(ppid, current_pid, "Parent PID should be different from current PID");
            }
            Ok(None) => {
                // This could happen if we're testing as init process (unlikely)
                println!("Current process has no parent (possibly init)");
            }
            Err(e) => {
                panic!("Failed to get parent PID: {}", e);
            }
        }
    }
    
    #[test]
    fn test_get_parent_pid_invalid() {
        // Test with invalid PID
        let result = get_parent_pid(999999);
        assert!(result.is_err(), "Should fail for invalid PID");
    }
    
    #[test]
    fn test_get_parent_pid_init() {
        // Test with init process (PID 1)
        match get_parent_pid(1) {
            Ok(ppid_opt) => {
                // Init process typically has no parent or parent PID 0
                if let Some(ppid) = ppid_opt {
                    assert!(ppid >= 0, "Parent PID should be non-negative");
                }
            }
            Err(_) => {
                // Some systems might not allow querying init process info
                println!("Cannot query init process info (expected on some systems)");
            }
        }
    }
}
