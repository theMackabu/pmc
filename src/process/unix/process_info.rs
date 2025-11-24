use std::time::SystemTime;
#[cfg(target_os = "linux")]
use std::time::{Duration, UNIX_EPOCH};

pub fn get_process_name(pid: u32) -> Result<String, String> {
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
            return Err(format!("Failed to get process info for PID {}", pid));
        }

        let name = unsafe {
            std::ffi::CStr::from_ptr(proc_info.pbi_name.as_ptr())
                .to_string_lossy()
                .to_string()
        };

        Ok(name)
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let comm_path = format!("/proc/{}/comm", pid);
        fs::read_to_string(&comm_path)
            .map(|name| name.trim().to_string())
            .map_err(|e| format!("Failed to read process name: {}", e))
    }
}

pub fn get_process_start_time(_pid: u32) -> Result<SystemTime, String> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

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

    let ppid = parts[3]
        .parse::<i32>()
        .map_err(|e| format!("Failed to parse ppid: {}", e))?;

    if ppid == 0 {
        Ok(None) // No parent (e.g., init process)
    } else {
        Ok(Some(ppid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_parent_pid_current_process() {
        let current_pid = std::process::id() as i32;
        let parent_result = get_parent_pid(current_pid);

        assert!(parent_result.is_ok());

        if let Ok(Some(ppid)) = parent_result {
            assert!(ppid > 0);
            println!("Current process PID: {}, Parent PID: {}", current_pid, ppid);
        }
    }

    #[test]
    fn test_get_parent_pid_invalid() {
        let invalid_pid = 999999;
        let result = get_parent_pid(invalid_pid);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_parent_pid_init() {
        // Test with init process (PID 1) - should have no parent or parent 0
        if let Ok(parent) = get_parent_pid(1) {
            if let Some(ppid) = parent {
                assert_eq!(ppid, 0);
            }
        }
    }
}
