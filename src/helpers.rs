use chrono::{DateTime, Utc};
use colored::Colorize;
use core::fmt;
use once_cell::sync::Lazy;
use regex::Regex;

pub static SUCCESS: Lazy<colored::ColoredString> = Lazy::new(|| "[PMC]".green());
pub static FAIL: Lazy<colored::ColoredString> = Lazy::new(|| "[PMC]".red());
pub static WARN: Lazy<colored::ColoredString> = Lazy::new(|| "[PMC]".yellow());
pub static WARN_STAR: Lazy<colored::ColoredString> = Lazy::new(|| "*".yellow());

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

impl From<colored::ColoredString> for ColoredString {
    fn from(cs: colored::ColoredString) -> Self { ColoredString(cs) }
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
    const UNIT: f64 = 1024.0;
    const SUFFIX: [&str; 4] = ["b", "kb", "mb", "gb"];

    let size = bytes as f64;
    let base = size.log10() / UNIT.log10();

    if size <= 0.0 {
        return "0b".to_string();
    }

    let mut buffer = ryu::Buffer::new();
    let result = buffer.format((UNIT.powf(base - base.floor()) * 10.0).round() / 10.0).trim_end_matches(".0");

    [result, SUFFIX[base.floor() as usize]].join("")
}
