use crate::helpers::{format_duration, format_memory, ColoredString};
use crate::process::Runner;
use crate::structs::Args;

use colored::Colorize;
use global_placeholders::global;
use macros_rs::{string, ternary};
use psutil::process::{MemoryInfo, Process};
use serde_json::json;
use std::env;

use tabled::{
    settings::{
        object::Rows,
        style::{BorderColor, Style},
        themes::Colorization,
        Color,
    },
    Table, Tabled,
};

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

pub fn info(id: &usize) {
    let runner = Runner::new(global!("pmc.logs"));
    println!("{:?}", runner.info(*id));
}

pub fn list(format: &String) {
    let runner = Runner::new(global!("pmc.logs"));
    let mut processes: Vec<ProcessItem> = Vec::new();

    #[derive(Tabled, Debug)]
    struct ProcessItem {
        id: ColoredString,
        name: String,
        pid: String,
        uptime: String,
        status: ColoredString,
        cpu: String,
        mem: String,
    }

    impl serde::Serialize for ProcessItem {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let trimmed_json = json!({
                "id": &self.id.0.trim(),
                "name": &self.name.trim(),
                "pid": &self.pid.trim(),
                "uptime": &self.uptime.trim(),
                "status": &self.status.0.trim(),
                "cpu": &self.cpu.trim(),
                "mem": &self.mem.trim(),
            });
            trimmed_json.serialize(serializer)
        }
    }

    for (id, item) in runner.list() {
        let mut memory_usage: Option<MemoryInfo> = None;
        let mut cpu_percent: Option<f32> = None;

        if let Ok(mut process) = Process::new(item.pid as u32) {
            memory_usage = process.memory_info().ok();
            cpu_percent = process.cpu_percent().ok();
        }

        let cpu_percent = match cpu_percent {
            Some(percent) => format!("{percent}%"),
            None => string!("0%"),
        };

        let memory_usage = match memory_usage {
            Some(usage) => format_memory(usage.rss()),
            None => string!("0b"),
        };

        processes.push(ProcessItem {
            id: ColoredString(id.cyan().bold()),
            pid: format!("{}   ", item.pid),
            cpu: format!("{cpu_percent}   "),
            mem: format!("{memory_usage}   "),
            name: format!("{}   ", item.name.clone()),
            status: ColoredString(ternary!(item.running, "online   ".green().bold(), "stopped   ".red().bold())),
            uptime: ternary!(item.running, format!("{}   ", format_duration(item.started)), string!("none")),
        });
    }

    let mut table = Table::new(&processes);
    table
        .with(Style::rounded().remove_verticals())
        .with(BorderColor::filled(Color::FG_BRIGHT_BLACK))
        .with(Colorization::exact([Color::FG_BRIGHT_CYAN], Rows::first()));

    if let Ok(json) = serde_json::to_string(&processes) {
        match format.as_str() {
            "raw" => println!("{:?}", processes),
            "json" => println!("{json}"),
            _ => println!("{}", table.to_string()),
        };
    };
}
