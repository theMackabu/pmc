use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use global_placeholders::global;
use macros_rs::crashln;
use pmc::{file::Exists, helpers};
use std::{fs, io};

pub fn exists() -> bool { fs::metadata(global!("pmc.pid")).is_ok() }
pub fn running(pid: i32) -> bool { unsafe { libc::kill(pid, 0) == 0 } }

pub fn uptime() -> io::Result<DateTime<Utc>> {
    let metadata = fs::metadata(global!("pmc.pid"))?;
    let creation_time = metadata.created()?;
    let creation_time = DateTime::from(creation_time);

    Ok(creation_time)
}

pub fn read() -> Result<i32> {
    let pid = fs::read_to_string(global!("pmc.pid")).map_err(|err| anyhow!(err))?;

    let trimmed_pid = pid.trim();
    let parsed_pid = trimmed_pid.parse::<i32>().map_err(|err| anyhow!(err))?;

    Ok(parsed_pid)
}

pub fn write(pid: u32) {
    if let Err(err) = fs::write(global!("pmc.pid"), pid.to_string()) {
        crashln!("{} Failed to write PID to file: {}", *helpers::FAIL, err);
    }
}

pub fn remove() {
    if Exists::check(&global!("pmc.pid")).file() {
        log::warn!("Stale PID file detected. Removing the PID file.");
        if let Err(err) = fs::remove_file(global!("pmc.pid")) {
            crashln!("{} Failed to remove PID file: {}", *helpers::FAIL, err);
        }
    } else {
        log::info!("No Stale PID file detected.");
    }
}

#[cfg(target_os = "linux")]
pub fn name(new_name: &str) {
    use std::ffi::CString;

    let pid = std::process::id() as libc::pid_t;
    let name_cstr = CString::new(new_name).expect("Failed to convert name to CString");

    unsafe {
        libc::setpgid(pid, 0);
        libc::prctl(libc::PR_SET_NAME, name_cstr.as_ptr() as libc::c_ulong, 0, 0, 0);
    }
}

#[cfg(target_os = "macos")]
pub fn name(new_name: &str) {
    use std::ffi::CString;

    let pid = std::process::id() as libc::pid_t;
    let name_cstr = CString::new(new_name).expect("Failed to convert name to CString");

    unsafe {
        libc::setpgid(pid, 0);
        libc::pthread_setname_np(name_cstr.as_ptr());
    }
}
