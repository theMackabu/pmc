use chrono::Local;
use global_placeholders::global;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};

pub struct Logger {
    file: File,
}

impl Logger {
    pub fn new() -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(global!("pmc.daemon.log"))?;
        Ok(Logger { file })
    }

    pub fn write(&mut self, message: &str) { writeln!(&mut self.file, "[{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), message).unwrap() }
}

#[macro_export]
macro_rules! log {
    ($message:expr $(, $arg:expr)*) => {
        let mut log = log::Logger::new().unwrap();
        log.write(format!($message $(, $arg)*).as_str());
    };
}
