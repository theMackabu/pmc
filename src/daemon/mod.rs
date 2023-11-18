mod fork;
mod pid;

use crate::helpers;
use crate::process::Runner;

use fork::{daemon, Fork};
use global_placeholders::global;
use macros_rs::crashln;
use std::{fs, process, thread::sleep, time::Duration};

extern "C" fn handle_termination_signal(_: libc::c_int) {
    pid::remove();
    unsafe { libc::_exit(0) }
}

pub fn restore() {
    println!("{} Spawning PMC daemon with pmc_home={}", *helpers::SUCCESS, global!("pmc.base"));
    if pid::exists() {
        match fs::read_to_string(global!("pmc.pid")) {
            Ok(pid) => {
                if let Ok(pid) = pid.trim().parse::<i32>() {
                    if !pid::running(pid) {
                        pid::remove()
                    } else {
                        crashln!("{} The daemon is already running with PID {pid}", *helpers::FAIL)
                    }
                }
            }
            Err(err) => crashln!("{} Failed to read existing PID from file: {err}", *helpers::FAIL),
        }
    }

    println!("{} PMC Successfully daemonized", *helpers::SUCCESS);
    match daemon(false, false) {
        Ok(Fork::Parent(_)) => {}
        Ok(Fork::Child) => {
            unsafe { libc::signal(libc::SIGTERM, handle_termination_signal as usize) };
            pid::write(process::id());

            loop {
                let runner = Runner::new();
                if !runner.list().is_empty() {
                    for (id, item) in runner.list() {
                        let mut runner = Runner::new();
                        let name = &Some(item.name.clone());

                        println!("name: {}, pid:{}, running: {}", item.name, item.pid, item.running);

                        if let Ok(id) = id.trim().parse::<usize>() {
                            if item.running {
                                println!("running_pid: {}, id: {id}", pid::running(item.pid as i32));
                                if !pid::running(item.pid as i32) {
                                    println!("restarted {}", item.pid);
                                    runner.restart(id, name);
                                }
                            }
                        }
                    }
                }

                sleep(Duration::from_secs(1));
            }
        }
        Err(err) => {
            crashln!("{} Daemon creation failed with code {err}", *helpers::FAIL)
        }
    }
}
