use global_placeholders::global;
use std::{ffi::CString, process::exit};

#[allow(dead_code)]
pub enum Fork {
    Parent(libc::pid_t),
    Child,
}

pub fn chdir() -> Result<libc::c_int, i32> {
    let dir = CString::new(global!("pmc.base")).expect("CString::new failed");
    let res = unsafe { libc::chdir(dir.as_ptr()) };
    match res {
        -1 => Err(-1),
        res => Ok(res),
    }
}

pub fn fork() -> Result<Fork, i32> {
    let res = unsafe { libc::fork() };
    match res {
        -1 => Err(-1),
        0 => Ok(Fork::Child),
        res => Ok(Fork::Parent(res)),
    }
}

pub fn setsid() -> Result<libc::pid_t, i32> {
    let res = unsafe { libc::setsid() };
    match res {
        -1 => Err(-1),
        res => Ok(res),
    }
}

pub fn close_fd() -> Result<i32, i32> {
    let mut res = false;
    for i in 0..=2 {
        res |= unsafe { libc::close(i) } == -1;
    }

    match res {
        true => Err(-1),
        false => Ok(1),
    }
}

pub fn daemon(nochdir: bool, noclose: bool) -> Result<Fork, i32> {
    match fork() {
        Ok(Fork::Parent(_)) => exit(0),
        Ok(Fork::Child) => setsid().and_then(|_| {
            if !nochdir {
                chdir()?;
            }
            if !noclose {
                close_fd()?;
            }
            fork()
        }),
        Err(n) => Err(n),
    }
}
