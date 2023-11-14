use std::{thread, time::Duration};

#[cxx::bridge]
pub mod process {
    unsafe extern "C++" {
        include!("pmc/src/include/process.h");
        include!("pmc/src/include/bridge.h");

        pub fn stop(pid: u64) -> u64;
        pub fn run(name: &str, log_path: &str, command: &str) -> u64;
    }
}

fn main() {
    let name = "example";
    let log_path = "tests/logs";
    let command = "node tests/index.js";
    let pid = process::run(&name, &log_path, &command);

    println!("pid: {pid}");
    thread::sleep(Duration::from_millis(1000));
    process::stop(pid);
}
