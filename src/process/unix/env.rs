use std::ffi::{CStr, OsString};
use std::os::unix::prelude::OsStringExt;

pub struct Vars {
    inner: std::vec::IntoIter<OsString>,
}

impl Iterator for Vars {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        self.inner.next().map(|var| var.into_string().unwrap())
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[cfg(target_os = "macos")]
unsafe fn environ() -> *mut *const *const libc::c_char {
    let environ = unsafe { libc::_NSGetEnviron() };
    environ as *mut *const *const libc::c_char
}

#[cfg(not(target_os = "macos"))]
unsafe fn environ() -> *mut *const *const libc::c_char {
    unsafe extern "C" {
        static mut environ: *const *const libc::c_char;
    }
    std::ptr::addr_of_mut!(environ)
}

pub fn env() -> Vec<String> {
    unsafe {
        let mut environ = *environ();
        let mut result = Vec::new();

        if !environ.is_null() {
            while !(*environ).is_null() {
                if let Some(key_value) = parse(CStr::from_ptr(*environ).to_bytes()) {
                    result.push(key_value);
                }
                environ = environ.add(1);
            }
        }

        return Vars {
            inner: result.into_iter(),
        }
        .collect();
    }

    fn parse(input: &[u8]) -> Option<OsString> {
        if input.is_empty() {
            return None;
        }
        Some(OsString::from_vec(input.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_parsing() {
        let env_vars = env();
        assert!(!env_vars.is_empty());
        // Most systems should have PATH environment variable
        assert!(env_vars.iter().any(|var| var.starts_with("PATH=")));
    }
}
