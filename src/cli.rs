use crate::helpers;
use crate::process::Runner;
use crate::structs::Args;

use global_placeholders::global;
use macros_rs::string;
use psutil::process::{MemoryInfo, Process};
use std::env;

pub fn get_version(short: bool) -> String {
    return match short {
        true => format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        false => format!("{} ({} {}) [{}]", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"), env!("BUILD_DATE"), env!("PROFILE")),
    };
}

pub fn start(name: &Option<String>, args: &Option<Args>) {
    let mut runner = Runner::new(global!("pmc.logs"));

    let name = match name {
        Some(name) => string!(name),
        None => string!(""),
    };

    match args {
        Some(Args::Id(id)) => runner.restart(*id),
        Some(Args::Script(script)) => runner.start(name, script),
        None => {}
    }
}

pub fn stop(id: &usize) {
    let mut runner = Runner::new(global!("pmc.logs"));
    runner.stop(*id);
    println!("Stopped process");
}

pub fn remove(id: &usize) {
    let mut runner = Runner::new(global!("pmc.logs"));
    runner.remove(*id);
    println!("Removed process");
}

pub fn list(format: &String) {
    let runner = Runner::new(global!("pmc.logs"));

    match format.as_str() {
        "raw" => println!("{:?}", runner.list()),
        "toml" => println!("{}", toml::to_string(runner.list()).unwrap()),
        "json" => println!("{}", serde_json::to_string(runner.list()).unwrap()),
        _ => {
            for (id, item) in runner.list() {
                let mut memory_usage: Option<MemoryInfo> = None;
                let mut cpu_percent: Option<f32> = None;

                if let Ok(mut process) = Process::new(item.pid as u32) {
                    memory_usage = process.memory_info().ok();
                    cpu_percent = process.cpu_percent().ok();
                }

                println!(
                    "id: {id}, name: {}, pid: {}, status: {}, uptime: {}, memory: {:?}, cpu: {:?}",
                    item.name,
                    item.pid,
                    item.running,
                    helpers::format_duration(item.started),
                    memory_usage,
                    cpu_percent
                );
            }
        }
    };
}
