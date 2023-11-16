use crate::process::Process;
use anyhow::Error;
use colored::Colorize;
use global_placeholders::global;
use macros_rs::{str, ternary};

use std::fs::File;
use std::io::BufRead;
use std::io::Seek;
use std::io::{self, BufReader, Read};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub fn logs(lines_to_tail: usize, log_file: &str, id: usize, log_type: &str, item_name: &str) {
    let file = File::open(log_file).unwrap();
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<io::Result<_>>().unwrap();
    let color = ternary!(log_type == "out", "green", "red");

    println!("{}", format!("\n{log_file} last {lines_to_tail} lines:").bright_black());

    let start_index = if lines.len() > lines_to_tail { lines.len() - lines_to_tail } else { 0 };
    for (i, line) in lines.iter().skip(start_index).enumerate() {
        let line_number = start_index + i;
        println!("{} {}", format!("{}|{} |", id, item_name).color(color), line);
    }
}

pub struct Exists;
impl Exists {
    pub fn folder(dir_name: String) -> Result<bool, Error> { Ok(Path::new(str!(dir_name)).is_dir()) }
    pub fn file(file_name: String) -> Result<bool, Error> { Ok(Path::new(str!(file_name)).exists()) }
}
