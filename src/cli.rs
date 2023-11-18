use crate::file;
use crate::helpers::{self, ColoredString};
use crate::process::Runner;
use crate::structs::Args;

use colored::Colorize;
use global_placeholders::global;
use macros_rs::{crashln, string, ternary};
use psutil::process::{MemoryInfo, Process};
use serde::Serialize;
use serde_json::json;
use std::env;

use tabled::{
    settings::{
        object::{Columns, Rows},
        style::{BorderColor, Style},
        themes::Colorization,
        Color, Rotate,
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
    let mut runner = Runner::new();

    match args {
        Some(Args::Id(id)) => {
            println!("{} Applying action restartProcess on ({id})", *helpers::SUCCESS);
            runner.restart(*id, name);

            println!("{} restarted ({id}) ✓", *helpers::SUCCESS);
            list(&string!(""));
        }
        Some(Args::Script(script)) => {
            let name = match name {
                Some(name) => string!(name),
                None => string!(script.split_whitespace().next().unwrap_or_default()),
            };

            println!("{} Creating process with ({name})", *helpers::SUCCESS);
            runner.start(name.clone(), script);

            println!("{} created ({name}) ✓", *helpers::SUCCESS);
            list(&string!(""));
        }
        None => {}
    }
}

pub fn stop(id: &usize) {
    println!("{} Applying action stopProcess on ({id})", *helpers::SUCCESS);
    let mut runner = Runner::new();
    runner.stop(*id);
    println!("{} stopped ({id}) ✓", *helpers::SUCCESS);
    list(&string!(""));
}

pub fn remove(id: &usize) {
    println!("{} Applying action removeProcess on ({id})", *helpers::SUCCESS);
    let mut runner = Runner::new();
    runner.remove(*id);
    println!("{} removed ({id}) ✓", *helpers::SUCCESS);
}

pub fn info(id: &usize) {
    let runner = Runner::new();

    #[derive(Tabled, Serialize)]
    struct Info {
        #[tabled(rename = "error log path ")]
        log_error: String,
        #[tabled(rename = "out log path")]
        log_out: String,
        #[tabled(rename = "cpu percent")]
        cpu_percent: String,
        #[tabled(rename = "memory usage")]
        memory_usage: String,
        #[tabled(rename = "script command ")]
        command: String,
        #[tabled(rename = "exec cwd")]
        path: String,
        #[tabled(rename = "script id")]
        id: String,
        uptime: String,
        pid: String,
        name: String,
        status: ColoredString,
    }

    if let Some(home) = home::home_dir() {
        if let Some(item) = runner.info(*id) {
            let mut memory_usage: Option<MemoryInfo> = None;
            let mut cpu_percent: Option<f32> = None;

            if let Ok(mut process) = Process::new(item.pid as u32) {
                memory_usage = process.memory_info().ok();
                cpu_percent = process.cpu_percent().ok();
            }

            let cpu_percent = match cpu_percent {
                Some(percent) => format!("{:.2}%", percent),
                None => string!("0%"),
            };

            let memory_usage = match memory_usage {
                Some(usage) => helpers::format_memory(usage.rss()),
                None => string!("0b"),
            };

            let path = file::make_relative(&item.path, &home)
                .map(|relative_path| relative_path.to_string_lossy().into_owned())
                .unwrap_or_else(|| crashln!("{} Unable to get your current directory", *helpers::FAIL));

            let data = vec![Info {
                cpu_percent,
                memory_usage,
                id: string!(id),
                name: item.name.clone(),
                command: format!("/bin/bash -c '{}'", item.script.clone()),
                path: format!("{} ", path),
                log_out: global!("pmc.logs.out", item.name.as_str()),
                log_error: global!("pmc.logs.error", item.name.as_str()),
                pid: ternary!(item.running, format!("{}", item.pid), string!("n/a")),
                status: ColoredString(ternary!(item.running, "online".green().bold(), "stopped".red().bold())),
                uptime: ternary!(item.running, format!("{}", helpers::format_duration(item.started)), string!("none")),
            }];

            let table = Table::new(data)
                .with(Rotate::Left)
                .with(Style::rounded().remove_horizontals())
                .with(Colorization::exact([Color::FG_CYAN], Columns::first()))
                .with(BorderColor::filled(Color::FG_BRIGHT_BLACK))
                .to_string();

            println!("{}\n{table}\n", format!("Describing process with id ({id})").on_bright_white().black());
            println!(" {}", format!("Use `pmc logs {id} [--lines <num>]` to display logs").white());
            println!(" {}", format!("Use `pmc env {id}`  to display environment variables").white());
        } else {
            crashln!("{} Process ({id}) not found", *helpers::FAIL);
        }
    } else {
        crashln!("{} Impossible to get your home directory", *helpers::FAIL);
    }
}

pub fn logs(id: &usize, lines: &usize) {
    let runner = Runner::new();

    if let Some(item) = runner.info(*id) {
        println!("{}", format!("Showing last {lines} lines for process [{id}] (change the value with --lines option)").yellow());

        let log_error = global!("pmc.logs.error", item.name.as_str());
        let log_out = global!("pmc.logs.out", item.name.as_str());

        file::logs(*lines, &log_error, *id, "error", &item.name);
        file::logs(*lines, &log_out, *id, "out", &item.name);
    } else {
        crashln!("{} Process ({id}) not found", *helpers::FAIL);
    }
}

#[cfg(target_os = "macos")]
pub fn env(id: &usize) {
    let runner = Runner::new();

    if let Some(item) = runner.info(*id) {
        for (key, value) in item.env.iter() {
            println!("{}: {}", key, value.green());
        }
    } else {
        crashln!("{} Process ({id}) not found", *helpers::FAIL);
    }
}

pub fn list(format: &String) {
    let runner = Runner::new();
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

    if runner.list().is_empty() {
        println!("{} Process table empty", *helpers::SUCCESS);
    } else {
        for (id, item) in runner.list() {
            let mut memory_usage: Option<MemoryInfo> = None;
            let mut cpu_percent: Option<f32> = None;

            if let Ok(mut process) = Process::new(item.pid as u32) {
                memory_usage = process.memory_info().ok();
                cpu_percent = process.cpu_percent().ok();
            }

            let cpu_percent = match cpu_percent {
                Some(percent) => format!("{:.0}%", percent),
                None => string!("0%"),
            };

            let memory_usage = match memory_usage {
                Some(usage) => helpers::format_memory(usage.rss()),
                None => string!("0b"),
            };

            processes.push(ProcessItem {
                id: ColoredString(id.cyan().bold()),
                pid: ternary!(item.running, format!("{}  ", item.pid), string!("n/a  ")),
                cpu: format!("{cpu_percent}   "),
                mem: format!("{memory_usage}   "),
                name: format!("{}   ", item.name.clone()),
                status: ColoredString(ternary!(item.running, "online   ".green().bold(), "stopped   ".red().bold())),
                uptime: ternary!(item.running, format!("{}  ", helpers::format_duration(item.started)), string!("none  ")),
            });
        }

        let table = Table::new(&processes)
            .with(Style::rounded().remove_verticals())
            .with(BorderColor::filled(Color::FG_BRIGHT_BLACK))
            .with(Colorization::exact([Color::FG_BRIGHT_CYAN], Rows::first()))
            .to_string();

        if let Ok(json) = serde_json::to_string(&processes) {
            match format.as_str() {
                "raw" => println!("{:?}", processes),
                "json" => println!("{json}"),
                _ => println!("{table}"),
            };
        };
    }
}
