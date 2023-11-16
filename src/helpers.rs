use anyhow::Error;
use chrono::{DateTime, Utc};
use macros_rs::str;
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Deserialize, Serialize)]
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

pub fn format_duration(datetime: DateTime<Utc>) -> String {
    let current_time = Utc::now();
    let duration = current_time.signed_duration_since(datetime);

    match duration.num_seconds() {
        s if s >= 86400 => format!("{}d", s / 86400),
        s if s >= 3600 => format!("{}h", s / 3600),
        s if s >= 60 => format!("{}m", s / 60),
        s => format!("{}s", s),
    }
}
