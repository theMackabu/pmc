use anyhow::Error;
use macros_rs::str;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Id {
    pub counter: AtomicUsize,
}

impl Id {
    pub fn new(start: usize) -> Self { Id { counter: AtomicUsize::new(start) } }
    pub fn next(&self) -> usize { self.counter.fetch_add(1, Ordering::SeqCst) }
}

pub struct Exists;
impl Exists {
    pub fn folder(dir_name: String) -> Result<bool, Error> { Ok(Path::new(str!(dir_name)).is_dir()) }
    pub fn file(file_name: String) -> Result<bool, Error> { Ok(Path::new(str!(file_name)).exists()) }
}
