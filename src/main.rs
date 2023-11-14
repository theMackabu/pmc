use std::{thread, time::Duration};

#[cxx::bridge]
mod cmd {
    unsafe extern "C++" {
        include!("pmc/src/include/cmd.h");
        fn run_command(name: &str, log_path: &str, command: &str) -> u64;
        fn kill_pid(pid: u64) -> u64;
    }
}

fn main() {
    let name = "example";
    let log_path = "tests/logs";
    let command = "node tests/index.js";
    let pid = cmd::run_command(&name, &log_path, &command);

    println!("pid: {pid}");
    thread::sleep(Duration::from_millis(1000));
    cmd::kill_pid(pid);
}
