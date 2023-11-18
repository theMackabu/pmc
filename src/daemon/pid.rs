use crate::helpers;
use global_placeholders::global;
use macros_rs::crashln;
use std::fs;

pub fn exists() -> bool { fs::metadata(global!("pmc.pid")).is_ok() }
pub fn running(pid: i32) -> bool { unsafe { libc::kill(pid, 0) == 0 } }

pub fn write(pid: i32) {
    if let Err(err) = fs::write(global!("pmc.pid"), pid.to_string()) {
        crashln!("{} Failed to write PID to file: {}", *helpers::FAIL, err);
    }
}

pub fn remove() {
    log::debug!("Stale PID file detected. Removing the PID file.");
    if let Err(err) = fs::remove_file(global!("pmc.pid")) {
        crashln!("{} Failed to remove PID file: {}", *helpers::FAIL, err);
    }
}
