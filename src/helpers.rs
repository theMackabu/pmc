use chrono::{DateTime, Utc};
use colored::Colorize;
use core::fmt;
use once_cell::sync::Lazy;
use regex::Regex;

pub static SUCCESS: Lazy<colored::ColoredString> = Lazy::new(|| "[PMC]".green());
pub static FAIL: Lazy<colored::ColoredString> = Lazy::new(|| "[PMC]".red());

#[derive(Clone, Debug)]
pub struct ColoredString(pub colored::ColoredString);

impl serde::Serialize for ColoredString {
    fn serialize<S: serde::ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let re = Regex::new(r"\x1B\[([0-9;]+)m").unwrap();
        let colored_string = &self.0;
        let stripped_string = re.replace_all(colored_string, "").to_string();
        serializer.serialize_str(&stripped_string)
    }
}

impl fmt::Display for ColoredString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
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

pub fn format_memory(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    match bytes {
        0..=KB if bytes < KB => format!("{}b", bytes),
        KB..=MB if bytes < MB => format!("{:.1}kb", bytes as f64 / KB as f64),
        MB..=GB if bytes < GB => format!("{:.1}mb", bytes as f64 / MB as f64),
        _ => format!("{:.1}gb", bytes as f64 / GB as f64),
    }
}
