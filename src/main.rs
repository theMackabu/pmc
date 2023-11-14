mod id;
mod process;

use crate::process::Runner;
use macros_rs::ternary;
use std::{thread, time::Duration};

#[cxx::bridge]
pub mod service {
    unsafe extern "C++" {
        include!("pmc/src/include/process.h");
        include!("pmc/src/include/bridge.h");

        pub fn stop(pid: i64) -> i64;
        pub fn run(name: &str, log_path: &str, command: &str) -> i64;
    }
}

fn main() {
    // save in .pmc/dump.toml
    // use global placeholders for home crate
    // use psutil for memory and cpu usage (in PAW)
    // create log dir if not exist
    // use clap cli and rataui for ui
    //    (pmc ls, pmc list, pmc ls --json, pmc list --json)
    //    [use clap command alias]

    let mut runner = Runner::new("tests/logs");

    runner.start("example", "node tests/index.js");
    println!("{:?}", runner.info(0));

    thread::sleep(Duration::from_millis(1000));

    runner.stop(0);
    println!("{:?}", runner.info(0));

    // runner.list().iter().for_each(|(id, item)| println!("id: {}\nname: {}", id, item.name));

    for (id, item) in runner.list() {
        println!("id: {id}\nname: {}\npid: {}\nstatus: {}", item.name, item.pid, ternary!(item.running, "online", "offline"));
    }
}
