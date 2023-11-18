use global_placeholders::global;
use std::ffi::CString;

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

pub fn close_fd() -> Result<(), i32> {
    match unsafe { libc::close(0) } {
        -1 => Err(-1),
        _ => match unsafe { libc::close(1) } {
            -1 => Err(-1),
            _ => match unsafe { libc::close(2) } {
                -1 => Err(-1),
                _ => Ok(()),
            },
        },
    }
}
