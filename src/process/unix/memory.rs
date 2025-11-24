/// Native memory info structure to replace psutil's MemoryInfo
#[derive(Debug, Clone)]
pub struct NativeMemoryInfo {
    pub rss: u64, // Resident Set Size
    pub vms: u64, // Virtual Memory Size
}

impl NativeMemoryInfo {
    pub fn rss(&self) -> u64 {
        self.rss
    }
    pub fn vms(&self) -> u64 {
        self.vms
    }
}

pub fn get_memory_info(pid: u32) -> Result<NativeMemoryInfo, String> {
    #[cfg(target_os = "macos")]
    {
        use std::mem;

        // macOS specific constants
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
