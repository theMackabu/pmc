mod fork;
mod pid;

use crate::helpers;
use crate::process::Runner;

use fork::{chdir, close_fd, fork, setsid, Fork};
use global_placeholders::global;
use macros_rs::crashln;
use std::{fs, thread::sleep, time::Duration};

extern "C" fn handle_termination_signal(_: libc::c_int) {
    pid::remove();
    unsafe {
        libc::_exit(0);
    }
}

pub fn start<F: Fn(i32)>(commands: F) {
    if pid::exists() {
        match fs::read_to_string(global!("pmc.pid")) {
            Ok(pid) => {
                if let Ok(pid) = pid.trim().parse::<i32>() {
                    match pid::running(pid) {
                        true => commands(pid),
                        false => pid::remove(),
                    }
                }
            }
            Err(err) => crashln!("{} Failed to read existing PID from file: {err}", *helpers::FAIL),
        }
    }

    println!("{} Spawning PMC daemon with pmc_home={}", *helpers::SUCCESS, global!("pmc.base"));
    match fork() {
        Ok(Fork::Child) => {
            unsafe { libc::signal(libc::SIGTERM, handle_termination_signal as usize) };

            match setsid() {
                Ok(pid) => {
                    log::trace!("Child process started with PID {}", pid);
                    println!("{} PMC Successfully daemonized", *helpers::SUCCESS);
                    pid::write(pid);
                }
                Err(err) => crashln!("{} setsid() failed with error: {}", *helpers::FAIL, err),
            }

            // if let Err(err) = chdir().and_then(|_| close_fd()).and_then(|_| fork().map_err(|err| err)) {
            //     crashln!("{} Daemon creation failed with error: {}", *helpers::FAIL, err);
            // }

            match chdir().and_then(|_| fork().map_err(|err| err)) {
                Ok(_status) => loop {
                    let runner = Runner::new();
                    if !runner.list().is_empty() {
                        for (id, item) in runner.list() {
                            let mut runner = Runner::new();
                            let name = &Some(item.name.clone());

                            if let Ok(id) = id.trim().parse::<usize>() {
                                if item.running {
                                    if !pid::running(item.pid as i32) {
                                        println!("restarted {}", item.pid);
                                        runner.restart(id, name);
                                    }
                                }
                            }
                        }
                    }

                    sleep(Duration::from_secs(5));
                },
                Err(err) => crashln!("{} Daemon creation failed with error: {}", *helpers::FAIL, err),
            }
        }
        Ok(Fork::Parent(pid)) => commands(pid),
        Err(err) => crashln!("{} Daemon creation failed with error code: {}", *helpers::FAIL, err),
    }
}
