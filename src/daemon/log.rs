use chrono::Local;
use global_placeholders::global;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};

pub struct Logger {
    file: File,
}

impl Logger {
    pub fn new() -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(global!("pmc.daemon.log"))?;
        Ok(Logger { file })
    }

    pub fn write(&mut self, message: &str, args: HashMap<String, String>) {
        let args = args
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<String>>()
            .join(", ");
        let msg = format!("{message} ({args})");

        log::info!("{msg}");
        writeln!(
            &mut self.file,
            "[{}] {msg}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.3f")
        )
        .unwrap()
    }
}

#[macro_export]
macro_rules! log {
    ($msg:expr, $($key:expr => $value:expr),* $(,)?) => {{
        let mut args = std::collections::HashMap::new();
        $(args.insert($key.to_string(), format!("{}", $value));)*
        crate::daemon::log::Logger::new().unwrap().write($msg, args)
    }}
}
