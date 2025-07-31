pub fn get_cpu_percent(pid: u32) -> f64 {
    #[cfg(target_os = "macos")]
    {
        use std::mem;
        use std::thread;
        use std::time::{Duration, Instant};
        
        // Use mach task info for better CPU measurement
        #[repr(C)]
        struct TaskBasicInfo {
            virtual_size: u32,
            resident_size: u32,
            resident_size_max: u32,
            user_time: TimeValue,
            system_time: TimeValue,
            policy: i32,
            suspend_count: i32,
        }
        
        #[repr(C)]
        struct TimeValue {
            seconds: i32,
            microseconds: i32,
        }
        
        const TASK_BASIC_INFO: u32 = 5;
        const TASK_BASIC_INFO_COUNT: u32 = 10;
        
        unsafe extern "C" {
            fn task_for_pid(
                target_tport: u32,
                pid: i32,
                task: *mut u32,
            ) -> i32;
            
            fn task_info(
                target_task: u32,
                flavor: u32,
                task_info_out: *mut libc::c_void,
                task_info_outCnt: *mut u32,
            ) -> i32;
            
            fn mach_task_self() -> u32;
        }
        
        let mut task: u32 = 0;
        let result = unsafe {
            task_for_pid(mach_task_self(), pid as i32, &mut task)
        };
        
        if result == 0 {
            let mut info: TaskBasicInfo = unsafe { mem::zeroed() };
            let mut count = TASK_BASIC_INFO_COUNT;
            
            let result = unsafe {
                task_info(
                    task,
                    TASK_BASIC_INFO,
                    &mut info as *mut _ as *mut libc::c_void,
                    &mut count,
                )
            };
            
            if result == 0 {
                // Take two measurements with a small interval
                let start_time = Instant::now();
                let start_user = info.user_time.seconds as f64 + info.user_time.microseconds as f64 / 1_000_000.0;
                let start_system = info.system_time.seconds as f64 + info.system_time.microseconds as f64 / 1_000_000.0;
                let start_total = start_user + start_system;
                
                // Wait a short time for measurement
                thread::sleep(Duration::from_millis(100));
                
                // Take second measurement
                let mut info2: TaskBasicInfo = unsafe { mem::zeroed() };
                let mut count2 = TASK_BASIC_INFO_COUNT;
                
                let result2 = unsafe {
                    task_info(
                        task,
                        TASK_BASIC_INFO,
                        &mut info2 as *mut _ as *mut libc::c_void,
                        &mut count2,
                    )
                };
                
                if result2 == 0 {
                    let elapsed_real = start_time.elapsed().as_secs_f64();
                    let end_user = info2.user_time.seconds as f64 + info2.user_time.microseconds as f64 / 1_000_000.0;
                    let end_system = info2.system_time.seconds as f64 + info2.system_time.microseconds as f64 / 1_000_000.0;
                    let end_total = end_user + end_system;
                    
                    let cpu_time_used = end_total - start_total;
                    
                    if elapsed_real > 0.0 {
                        let cpu_percent = (cpu_time_used / elapsed_real) * 100.0;
                        return cpu_percent.min(100.0 * num_cpus::get() as f64);
                    }
                }
            }
        }
        
        // Fallback: try to use a simpler approach with /usr/bin/ps
        if let Ok(output) = std::process::Command::new("ps")
            .args(&["-p", &pid.to_string(), "-o", "pcpu="])
            .output()
        {
            if let Ok(cpu_str) = String::from_utf8(output.stdout) {
                if let Ok(cpu) = cpu_str.trim().parse::<f64>() {
                    return cpu;
                }
            }
        }
        
        0.0
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::thread;
        use std::time::{Duration, Instant};
        
        // Take two measurements to calculate CPU usage rate
        let get_cpu_time = |pid: u32| -> Option<(f64, f64)> {
            let stat_path = format!("/proc/{}/stat", pid);
            if let Ok(stat_content) = fs::read_to_string(&stat_path) {
                let parts: Vec<&str> = stat_content.split_whitespace().collect();
                if parts.len() > 16 {
                    let utime = parts[13].parse::<u64>().ok()? as f64;
                    let stime = parts[14].parse::<u64>().ok()? as f64;
                    let total_process_time = (utime + stime) / 100.0; // Convert clock ticks to seconds
                    
                    // Get system CPU time
                    if let Ok(stat_content) = fs::read_to_string("/proc/stat") {
                        if let Some(cpu_line) = stat_content.lines().next() {
                            let cpu_parts: Vec<&str> = cpu_line.split_whitespace().collect();
                            if cpu_parts.len() > 7 {
                                let user: u64 = cpu_parts[1].parse().ok()?;
                                let nice: u64 = cpu_parts[2].parse().ok()?;
                                let system: u64 = cpu_parts[3].parse().ok()?;
                                let idle: u64 = cpu_parts[4].parse().ok()?;
                                let iowait: u64 = cpu_parts[5].parse().ok()?;
                                let irq: u64 = cpu_parts[6].parse().ok()?;
                                let softirq: u64 = cpu_parts[7].parse().ok()?;
                                
                                let total_system_time = (user + nice + system + idle + iowait + irq + softirq) as f64 / 100.0;
                                return Some((total_process_time, total_system_time));
                            }
                        }
                    }
                }
            }
            None
        };
        
        if let Some((start_process, start_system)) = get_cpu_time(pid) {
            let start_time = Instant::now();
            thread::sleep(Duration::from_millis(100));
            
            if let Some((end_process, end_system)) = get_cpu_time(pid) {
                let elapsed = start_time.elapsed().as_secs_f64();
                let process_diff = end_process - start_process;
                let system_diff = end_system - start_system;
                
                if system_diff > 0.0 && elapsed > 0.0 {
                    return (process_diff / system_diff) * 100.0 * num_cpus::get() as f64;
                }
            }
        }
        
        0.0
    }
} 