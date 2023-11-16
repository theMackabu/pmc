use crate::helpers;
use anyhow::Error;
use colored::Colorize;
use macros_rs::{crashln, str, ternary};

use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf, StripPrefixError},
};

pub fn logs(lines_to_tail: usize, log_file: &str, id: usize, log_type: &str, item_name: &str) {
    let file = File::open(log_file).unwrap();
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<io::Result<_>>().unwrap();
    let color = ternary!(log_type == "out", "green", "red");

    println!("{}", format!("\n{log_file} last {lines_to_tail} lines:").bright_black());

    let start_index = if lines.len() > lines_to_tail { lines.len() - lines_to_tail } else { 0 };
    for (_, line) in lines.iter().skip(start_index).enumerate() {
        println!("{} {}", format!("{}|{} |", id, item_name).color(color), line);
    }
}

pub fn cwd() -> PathBuf {
    match env::current_dir() {
        Ok(path) => path,
        Err(_) => crashln!("{} Unable to find current working directory", *helpers::FAIL),
    }
}

pub fn make_relative(current: &Path, home: &Path) -> Option<std::path::PathBuf> {
    match current.strip_prefix(home) {
        Ok(relative_path) => Some(Path::new("~").join(relative_path)),
        Err(StripPrefixError { .. }) => None,
    }
}

pub struct Exists;
impl Exists {
    pub fn folder(dir_name: String) -> Result<bool, Error> { Ok(Path::new(str!(dir_name)).is_dir()) }
    pub fn file(file_name: String) -> Result<bool, Error> { Ok(Path::new(str!(file_name)).exists()) }
}
