use crate::{helpers, log};
use anyhow::Error;
use colored::Colorize;
use macros_rs::{crashln, str, string, ternary};

use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf, StripPrefixError},
    thread::sleep,
    time::Duration,
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

// fix
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

pub fn raw(path: String) -> Vec<u8> {
    match fs::read(&path) {
        Ok(contents) => contents,
        Err(err) => crashln!("{} Cannot find dumpfile.\n{}", *helpers::FAIL, string!(err).white()),
    }
}

pub fn read<T: serde::de::DeserializeOwned>(path: String) -> T {
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) => crashln!("{} Cannot find dumpfile.\n{}", *helpers::FAIL, string!(err).white()),
    };

    match toml::from_str(&contents).map_err(|err| string!(err)) {
        Ok(parsed) => parsed,
        Err(err) => crashln!("{} Cannot parse dumpfile.\n{}", *helpers::FAIL, err.white()),
    }
}

pub fn from_rmp<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    match rmp_serde::from_slice(&bytes) {
        Ok(parsed) => parsed,
        Err(err) => crashln!("{} Cannot parse file.\n{}", *helpers::FAIL, string!(err).white()),
    }
}

pub fn read_rmp<T: serde::de::DeserializeOwned>(path: String) -> T {
    let mut retry_count = 0;
    let max_retries = 5;

    let bytes = loop {
        match fs::read(&path) {
            Ok(contents) => break contents,
            Err(err) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    log!("{} Cannot find file.\n{}", *helpers::FAIL, string!(err).white());
                } else {
                    log!("{} Error reading file. Retrying... (Attempt {}/{})", *helpers::FAIL, retry_count, max_retries);
                }
            }
        }
        sleep(Duration::from_secs(1));
    };

    retry_count = 0;

    loop {
        match rmp_serde::from_slice(&bytes) {
            Ok(parsed) => break parsed,
            Err(err) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    log!("{} Cannot parse file.\n{}", *helpers::FAIL, string!(err).white());
                } else {
                    log!("{} Error parsing file. Retrying... (Attempt {}/{})", *helpers::FAIL, retry_count, max_retries);
                }
            }
        }
        sleep(Duration::from_secs(1));
    }
}
