use colored::Colorize;
use global_placeholders::global;
use macros_rs::{crashln, string, ternary};
use psutil::process::{MemoryInfo, Process};
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::env;

use pmc::{
    config,
    file::{self, Exists},
    helpers::{self, ColoredString},
    log,
    process::Runner,
};

use tabled::{
    settings::{
        object::{Columns, Rows},
        style::{BorderColor, Style},
        themes::Colorization,
        Color, Modify, Rotate, Width,
    },
    Table, Tabled,
};

#[derive(Clone, Debug)]
pub enum Args {
    Id(usize),
    Script(String),
}

fn format(server_name: &String) -> (String, String) {
    let kind = ternary!(server_name == "internal", "", "remote ").to_string();
    return (kind, server_name.to_string());
}

pub fn get_version(short: bool) -> String {
    return match short {
        true => format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        false => format!("{} ({} {}) [{}]", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"), env!("BUILD_DATE"), env!("PROFILE")),
    };
}

pub fn start(name: &Option<String>, args: &Option<Args>, watch: &Option<String>, server_name: &String) {
    let mut runner = Runner::new();
    let config = config::read();
    let (kind, list_name) = format(server_name);

    match args {
        Some(Args::Id(id)) => {
            println!("{} Applying {kind}action restartProcess on ({id})", *helpers::SUCCESS);

            if *server_name == "internal" {
                let item = runner.get(*id);

                match watch {
                    Some(path) => item.watch(path),
                    None => item.disable_watch(),
                }

                name.as_ref().map(|n| item.rename(n.trim().replace("\n", "")));
                item.restart();

                log!("process started (id={id})");
            } else {
                let Some(servers) = config::servers().servers else {
                    crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL)
                };

                if let Some(server) = servers.get(server_name) {
                    match Runner::connect(server_name.clone(), server.clone(), false) {
                        Some(mut remote) => remote.get(*id).restart(),
                        None => crashln!("{} Failed to connect (name={server_name}, address={})", *helpers::FAIL, server.address),
                    };
                }
            }

            println!("{} restarted {kind}({id}) ✓", *helpers::SUCCESS);
            list(&string!("default"), &list_name);
        }
        Some(Args::Script(script)) => {
            let name = match name {
                Some(name) => string!(name),
                None => string!(script.split_whitespace().next().unwrap_or_default()),
            };
            if *server_name == "internal" {
                let pattern = Regex::new(r"(?m)^[a-zA-Z0-9]+(/[a-zA-Z0-9]+)*(\.js|\.ts)?$").unwrap();

                if pattern.is_match(script) {
                    let script = format!("{} {script}", config.runner.node);
                    runner.start(&name, &script, file::cwd(), watch).save();
                } else {
                    runner.start(&name, script, file::cwd(), watch).save();
                }

                log!("process created (name={name})");
            } else {
                let Some(servers) = config::servers().servers else {
                    crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL)
                };

                if let Some(server) = servers.get(server_name) {
                    match Runner::connect(server_name.clone(), server.clone(), false) {
                        Some(mut remote) => remote.start(&name, script, file::cwd(), watch),
                        None => crashln!("{} Failed to connect (name={server_name}, address={})", *helpers::FAIL, server.address),
                    };
                }
            }

            println!("{} Creating {kind}process with ({name})", *helpers::SUCCESS);

            println!("{} {kind}created ({name}) ✓", *helpers::SUCCESS);
            list(&string!("default"), &list_name);
        }
        None => {}
    }
}

pub fn stop(id: &usize, server_name: &String) {
    let (kind, list_name) = format(server_name);
    println!("{} Applying {kind}action stopProcess on ({id})", *helpers::SUCCESS);

    if *server_name == "internal" {
        let mut runner = Runner::new();
        runner.get(*id).stop();
        log!("process stopped (id={id})");
    } else {
        let Some(servers) = config::servers().servers else {
            crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL)
        };

        if let Some(server) = servers.get(server_name) {
            match Runner::connect(server_name.clone(), server.clone(), false) {
                Some(mut remote) => remote.get(*id).stop(),
                None => crashln!("{} Failed to connect (name={server_name}, address={})", *helpers::FAIL, server.address),
            };
        }
    }

    println!("{} stopped {kind}({id}) ✓", *helpers::SUCCESS);
    list(&string!("default"), &list_name);
}

pub fn remove(id: &usize, server_name: &String) {
    let (kind, _) = format(server_name);
    println!("{} Applying {kind}action removeProcess on ({id})", *helpers::SUCCESS);

    if *server_name == "internal" {
        Runner::new().remove(*id);
    } else {
        let Some(servers) = config::servers().servers else {
            crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL)
        };

        if let Some(server) = servers.get(server_name) {
            match Runner::connect(server_name.clone(), server.clone(), false) {
                Some(mut remote) => remote.remove(*id),
                None => crashln!("{} Failed to remove (name={server_name}, address={})", *helpers::FAIL, server.address),
            };
        }
    }

    println!("{} removed {kind}({id}) ✓", *helpers::SUCCESS);
    log!("process removed (id={id})");
}

pub fn info(id: &usize, format: &String) {
    #[derive(Clone, Debug, Tabled)]
    struct Info {
        #[tabled(rename = "error log path ")]
        log_error: String,
        #[tabled(rename = "out log path")]
        log_out: String,
        #[tabled(rename = "cpu percent")]
        cpu_percent: String,
        #[tabled(rename = "memory usage")]
        memory_usage: String,
        #[tabled(rename = "path hash")]
        hash: String,
        #[tabled(rename = "watching")]
        watch: String,
        #[tabled(rename = "exec cwd")]
        path: String,
        #[tabled(rename = "script command ")]
        command: String,
        #[tabled(rename = "script id")]
        id: String,
        restarts: u64,
        uptime: String,
        pid: String,
        name: String,
        status: ColoredString,
    }

    impl Serialize for Info {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let trimmed_json = json!({
                "id": &self.id.trim(),
                "pid": &self.pid.trim(),
                "name": &self.name.trim(),
                "path": &self.path.trim(),
                "restarts": &self.restarts,
                "watch": &self.watch.trim(),
                "watch": &self.hash.trim(),
                "uptime": &self.uptime.trim(),
                "status": &self.status.0.trim(),
                "log_out": &self.log_out.trim(),
                "cpu": &self.cpu_percent.trim(),
                "command": &self.command.trim(),
                "mem": &self.memory_usage.trim(),
                "log_error": &self.log_error.trim(),
            });

            trimmed_json.serialize(serializer)
        }
    }

    if let Some(home) = home::home_dir() {
        let config = config::read().runner;
        let item = Runner::new().get(*id).clone();
        let mut memory_usage: Option<MemoryInfo> = None;
        let mut cpu_percent: Option<f32> = None;

        if let Ok(mut process) = Process::new(item.pid as u32) {
            memory_usage = process.memory_info().ok();
            cpu_percent = process.cpu_percent().ok();
        }

        let cpu_percent =
            match cpu_percent {
                Some(percent) => format!("{:.2}%", percent),
                None => string!("0%"),
            };

        let memory_usage = match memory_usage {
            Some(usage) => helpers::format_memory(usage.rss()),
            None => string!("0b"),
        };

        let status =
            if item.running {
                "online   ".green().bold()
            } else {
                match item.crash.crashed {
                    true => "crashed   ",
                    false => "stopped   ",
                }
                .red()
                .bold()
            };

        let path = file::make_relative(&item.path, &home)
            .map(|relative_path| relative_path.to_string_lossy().into_owned())
            .unwrap_or_else(|| crashln!("{} Unable to get your current directory", *helpers::FAIL));

        let data = vec![Info {
            cpu_percent,
            memory_usage,
            id: string!(id),
            restarts: item.restarts,
            name: item.name.clone(),
            path: format!("{} ", path),
            status: ColoredString(status),
            log_out: global!("pmc.logs.out", item.name.as_str()),
            log_error: global!("pmc.logs.error", item.name.as_str()),
            pid: ternary!(item.running, format!("{}", item.pid), string!("n/a")),
            command: format!("{} {} '{}'", config.shell, config.args.join(" "), item.script),
            hash: ternary!(item.watch.enabled, format!("{}  ", item.watch.hash), string!("none  ")),
            watch: ternary!(item.watch.enabled, format!("{path}/{}  ", item.watch.path), string!("disabled  ")),
            uptime: ternary!(item.running, format!("{}", helpers::format_duration(item.started)), string!("none")),
        }];

        let table = Table::new(data.clone())
            .with(Rotate::Left)
            .with(Style::rounded().remove_horizontals())
            .with(Colorization::exact([Color::FG_CYAN], Columns::first()))
            .with(BorderColor::filled(Color::FG_BRIGHT_BLACK))
            .to_string();

        if let Ok(json) = serde_json::to_string(&data[0]) {
            match format.as_str() {
                "raw" => println!("{:?}", data[0]),
                "json" => println!("{json}"),
                _ => {
                    println!("{}\n{table}\n", format!("Describing process with id ({id})").on_bright_white().black());
                    println!(" {}", format!("Use `pmc logs {id} [--lines <num>]` to display logs").white());
                    println!(" {}", format!("Use `pmc env {id}`  to display environment variables").white());
                }
            };
        };
    } else {
        crashln!("{} Impossible to get your home directory", *helpers::FAIL);
    }
}

pub fn logs(id: &usize, lines: &usize) {
    let item = Runner::new().get(*id).clone();
    let log_error = global!("pmc.logs.error", item.name.as_str());
    let log_out = global!("pmc.logs.out", item.name.as_str());

    if Exists::file(log_error.clone()).unwrap() && Exists::file(log_out.clone()).unwrap() {
        println!("{}", format!("Showing last {lines} lines for process [{id}] (change the value with --lines option)").yellow());

        file::logs(*lines, &log_error, *id, "error", &item.name);
        file::logs(*lines, &log_out, *id, "out", &item.name);
    } else {
        crashln!("{} Logs for process ({id}) not found", *helpers::FAIL);
    }
}

pub fn env(id: &usize) {
    let item = Runner::new().get(*id).clone();
    for (key, value) in item.env.iter() {
        println!("{}: {}", key, value.green());
    }
}

pub fn list(format: &String, server_name: &String) {
    let render_list = |runner: &mut Runner| {
        let mut processes: Vec<ProcessItem> = Vec::new();

        #[derive(Tabled, Debug)]
        struct ProcessItem {
            id: ColoredString,
            name: String,
            pid: String,
            uptime: String,
            #[tabled(rename = "↺")]
            restarts: String,
            status: ColoredString,
            cpu: String,
            mem: String,
            #[tabled(rename = "watching")]
            watch: String,
        }

        impl serde::Serialize for ProcessItem {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let trimmed_json = json!({
                    "cpu": &self.cpu.trim(),
                    "mem": &self.mem.trim(),
                    "id": &self.id.0.trim(),
                    "pid": &self.pid.trim(),
                    "name": &self.name.trim(),
                    "watch": &self.watch.trim(),
                    "uptime": &self.uptime.trim(),
                    "status": &self.status.0.trim(),
                    "restarts": &self.restarts.trim(),
                });
                trimmed_json.serialize(serializer)
            }
        }

        if runner.is_empty() {
            println!("{} Process table empty", *helpers::SUCCESS);
        } else {
            for (id, item) in runner.items() {
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

                let status = if item.running {
                    "online   ".green().bold()
                } else {
                    match item.crash.crashed {
                        true => "crashed   ",
                        false => "stopped   ",
                    }
                    .red()
                    .bold()
                };

                processes.push(ProcessItem {
                    status: ColoredString(status),
                    cpu: format!("{cpu_percent}   "),
                    mem: format!("{memory_usage}   "),
                    restarts: format!("{}  ", item.restarts),
                    name: format!("{}   ", item.name.clone()),
                    id: ColoredString(id.to_string().cyan().bold()),
                    pid: ternary!(item.running, format!("{}  ", item.pid), string!("n/a  ")),
                    watch: ternary!(item.watch.enabled, format!("{}  ", item.watch.path), string!("disabled  ")),
                    uptime: ternary!(item.running, format!("{}  ", helpers::format_duration(item.started)), string!("none  ")),
                });
            }

            let table = Table::new(&processes)
                .with(Style::rounded().remove_verticals())
                .with(BorderColor::filled(Color::FG_BRIGHT_BLACK))
                .with(Colorization::exact([Color::FG_BRIGHT_CYAN], Rows::first()))
                .with(Modify::new(Columns::single(1)).with(Width::truncate(35).suffix("...  ")))
                .to_string();

            if let Ok(json) = serde_json::to_string(&processes) {
                match format.as_str() {
                    "raw" => println!("{:?}", processes),
                    "json" => println!("{json}"),
                    "default" => println!("{table}"),
                    _ => {}
                };
            };
        }
    };

    if let Some(servers) = config::servers().servers {
        let mut failed: Vec<(String, String)> = vec![];

        if let Some(server) = servers.get(server_name) {
            match Runner::connect(server_name.clone(), server.clone(), true) {
                Some(mut remote) => render_list(&mut remote),
                None => println!("{} Failed to fetch (name={server_name}, address={})", *helpers::FAIL, server.address),
            }
        } else {
            if matches!(&**server_name, "internal" | "all") {
                println!("{} Internal daemon", *helpers::SUCCESS);
                render_list(&mut Runner::new());
            } else {
                crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL);
            }
        }

        if *server_name == "all" {
            for (name, server) in servers {
                match Runner::connect(name.clone(), server.clone(), true) {
                    Some(mut remote) => render_list(&mut remote),
                    None => failed.push((name, server.address)),
                }
            }
        }

        if !failed.is_empty() {
            println!("{} Failed servers:", *helpers::FAIL);
            failed
                .iter()
                .for_each(|server| println!(" {} {} {}", "-".yellow(), format!("{}", server.0), format!("[{}]", server.1).white()));
        }
    } else {
        render_list(&mut Runner::new());
    }
}
