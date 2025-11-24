use crate::{helpers, log, process::Process};
use colored::Colorize;
use macros_rs::{crashln, string, ternary};

use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

pub fn logs(item: &Process, lines_to_tail: usize, kind: &str) {
    let log_file = match kind {
        "out" => item.logs().out,
        "error" => item.logs().error,
        _ => item.logs().out,
    };

    if !Exists::check(&log_file).empty() {
        let file = File::open(&log_file).unwrap();
        let reader = BufReader::new(file);
        let lines = reader
            .lines()
            .map(|line| line.unwrap_or_else(|err| format!("error reading line: {err}")))
            .collect();

        logs_internal(lines, lines_to_tail, &log_file, item.id, kind, &item.name)
    } else {
        println!("{} No logs found in {log_file}", *helpers::FAIL)
    }
}

pub fn logs_internal(
    lines: Vec<String>,
    lines_to_tail: usize,
    log_file: &str,
    id: usize,
    log_type: &str,
    item_name: &str,
) {
    println!(
        "{}",
        format!("\n{log_file} last {lines_to_tail} lines:").bright_black()
    );

    let color = ternary!(log_type == "out", "green", "red");
    let start_index = if lines.len() > lines_to_tail {
        lines.len() - lines_to_tail
    } else {
        0
    };

    for (_, line) in lines.iter().skip(start_index).enumerate() {
        println!(
            "{} {}",
            format!("{}|{} |", id, item_name).color(color),
            line
        );
    }
}

pub fn cwd() -> PathBuf {
    match env::current_dir() {
        Ok(path) => path,
        Err(_) => crashln!(
            "{} Unable to find current working directory",
            *helpers::FAIL
        ),
    }
}

pub fn make_relative(current: &Path, home: &Path) -> PathBuf {
    match current.strip_prefix(home) {
        Err(_) => Path::new(home).join(current),
        Ok(relative_path) => Path::new("~").join(relative_path),
    }
}

pub struct Exists<'p> {
    path: &'p str,
}

impl<'p> Exists<'p> {
    pub fn check(path: &'p str) -> Self {
        Self { path }
    }
    pub fn folder(&self) -> bool {
        Path::new(self.path).is_dir()
    }
    pub fn file(&self) -> bool {
        Path::new(self.path).exists()
    }
    pub fn empty(&self) -> bool {
        fs::metadata(Path::new(self.path))
            .map(|m| m.len() == 0)
            .unwrap_or(true)
    }
}

pub fn raw(path: String) -> Vec<u8> {
    match fs::read(&path) {
        Ok(contents) => contents,
        Err(err) => crashln!(
            "{} Cannot find dumpfile.\n{}",
            *helpers::FAIL,
            string!(err).white()
        ),
    }
}

pub fn read<T: serde::de::DeserializeOwned>(path: String) -> T {
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) => crashln!(
            "{} Cannot find dumpfile.\n{}",
            *helpers::FAIL,
            string!(err).white()
        ),
    };

    match toml::from_str(&contents).map_err(|err| string!(err)) {
        Ok(parsed) => parsed,
        Err(err) => crashln!("{} Cannot parse dumpfile.\n{}", *helpers::FAIL, err.white()),
    }
}

pub fn from_object<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    match ron::de::from_bytes(&bytes) {
        Ok(parsed) => parsed,
        Err(err) => crashln!(
            "{} Cannot parse file.\n{}",
            *helpers::FAIL,
            string!(err).white()
        ),
    }
}

pub fn read_object<T: serde::de::DeserializeOwned>(path: String) -> T {
    let mut retry_count = 0;
    let max_retries = 5;

    let bytes = loop {
        match fs::read(&path) {
            Ok(contents) => break contents,
            Err(err) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    log!("file::read] Cannot find file: {err}");
                    println!(
                        "{} Cannot find file.\n{}",
                        *helpers::FAIL,
                        string!(err).white()
                    );
                } else {
                    log!(
                        "file::read] Error reading file. Retrying... (Attempt {retry_count}/{max_retries})"
                    );
                    println!(
                        "{} Error reading file. Retrying... (Attempt {retry_count}/{max_retries})",
                        *helpers::FAIL
                    );
                }
            }
        }
        sleep(Duration::from_secs(1));
    };

    retry_count = 0;

    loop {
        match ron::de::from_bytes(&bytes) {
            Ok(parsed) => break parsed,
            Err(err) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    log!("[file::parse] Cannot parse file: {err}");
                    println!(
                        "{} Cannot parse file.\n{}",
                        *helpers::FAIL,
                        string!(err).white()
                    );
                } else {
                    log!(
                        "[file::parse] Error parsing file. Retrying... (Attempt {retry_count}/{max_retries})"
                    );
                    println!(
                        "{} Error parsing file. Retrying... (Attempt {retry_count}/{max_retries})",
                        *helpers::FAIL
                    );
                }
            }
        }
        sleep(Duration::from_secs(1));
    }
}
