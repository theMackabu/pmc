use super::NativeProcess;

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
            p_un: [u8; 16], // Union, simplified as byte array
            p_vmspace: u64,
            p_sigacts: u64,
            p_flag: libc::c_int,
            p_stat: libc::c_char,
            p_pid: libc::pid_t,
            p_oppid: libc::pid_t,
            p_dupfd: libc::c_int,
            // ... other fields we don't need, represented as padding
            _padding: [u8; 400], // Simplified padding to match struct size
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
        let proc_dir = fs::read_dir("/proc").map_err(|e| format!("Failed to read /proc: {}", e))?;

        for entry in proc_dir {
            if let Ok(entry) = entry
                && let Ok(file_name) = entry.file_name().into_string()
                && let Ok(pid) = file_name.parse::<u32>()
                && let Ok(process) = NativeProcess::new(pid)
            {
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
