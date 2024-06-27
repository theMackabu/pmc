pub(crate) mod internal;
pub(crate) mod server;

use colored::Colorize;
use internal::Internal;
use macros_rs::{crashln, string, ternary};
use psutil::process::{MemoryInfo, Process};
use serde::Serialize;
use serde_json::json;
use std::env;

use pmc::{
    config, file,
    helpers::{self, ColoredString},
    log,
    process::{http, ItemSingle, Runner},
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

#[derive(Clone, Debug)]
pub enum Item {
    Id(usize),
    Name(String),
}

fn format(server_name: &String) -> (String, String) {
    let kind = ternary!(matches!(&**server_name, "internal" | "local"), "", "remote ").to_string();
    return (kind, server_name.to_string());
}

pub fn get_version(short: bool) -> String {
    return match short {
        true => format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        false => match env!("GIT_HASH") {
            "" => format!("{} ({}) [{}]", env!("CARGO_PKG_VERSION"), env!("BUILD_DATE"), env!("PROFILE")),
            hash => format!("{} ({} {hash}) [{}]", env!("CARGO_PKG_VERSION"), env!("BUILD_DATE"), env!("PROFILE")),
        },
    };
}

pub fn start(name: &Option<String>, args: &Args, watch: &Option<String>, server_name: &String) {
    let runner = Runner::new();
    let (kind, list_name) = format(server_name);

    match args {
        Args::Id(id) => Internal { id: *id, runner, server_name, kind }.restart(name, watch),
        Args::Script(script) => match runner.find(&script) {
            Some(id) => Internal { id, runner, server_name, kind }.restart(name, watch),
            None => Internal { id: 0, runner, server_name, kind }.create(script, name, watch),
        },
    }

    list(&string!("default"), &list_name);
}

pub fn stop(item: &Item, server_name: &String) {
    let runner: Runner = Runner::new();
    let (kind, list_name) = format(server_name);

    match item {
        Item::Id(id) => Internal { id: *id, runner, server_name, kind }.stop(),
        Item::Name(name) => match runner.find(&name) {
            Some(id) => Internal { id, runner, server_name, kind }.stop(),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
    }

    list(&string!("default"), &list_name);
}

pub fn remove(item: &Item, server_name: &String) {
    let runner: Runner = Runner::new();
    let (kind, _) = format(server_name);

    match item {
        Item::Id(id) => Internal { id: *id, runner, server_name, kind }.remove(),
        Item::Name(name) => match runner.find(&name) {
            Some(id) => Internal { id, runner, server_name, kind }.remove(),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
    }
}

pub fn info(id: &usize, format: &String, server_name: &String) {
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
        children: String,
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
                "hash": &self.hash.trim(),
                "watch": &self.watch.trim(),
                "children": &self.children,
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

    let render_info = |data: Vec<Info>| {
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
    };

    if matches!(&**server_name, "internal" | "local") {
        if let Some(home) = home::home_dir() {
            let config = config::read().runner;
            let mut runner = Runner::new();
            let item = runner.process(*id);

            let mut memory_usage: Option<MemoryInfo> = None;
            let mut cpu_percent: Option<f32> = None;

            let path = file::make_relative(&item.path, &home).to_string_lossy().into_owned();
            let children = if item.children.is_empty() { "none".to_string() } else { format!("{:?}", item.children) };

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

            let data = vec![Info {
                children,
                cpu_percent,
                memory_usage,
                id: string!(id),
                restarts: item.restarts,
                name: item.name.clone(),
                log_out: item.logs().out,
                path: format!("{} ", path),
                log_error: item.logs().error,
                status: ColoredString(status),
                pid: ternary!(item.running, format!("{}", item.pid), string!("n/a")),
                command: format!("{} {} '{}'", config.shell, config.args.join(" "), item.script),
                hash: ternary!(item.watch.enabled, format!("{}  ", item.watch.hash), string!("none  ")),
                watch: ternary!(item.watch.enabled, format!("{path}/{}  ", item.watch.path), string!("disabled  ")),
                uptime: ternary!(item.running, format!("{}", helpers::format_duration(item.started)), string!("none")),
            }];

            render_info(data)
        } else {
            crashln!("{} Impossible to get your home directory", *helpers::FAIL);
        }
    } else {
        let data: (pmc::process::Process, Runner);
        let Some(servers) = config::servers().servers else {
            crashln!("{} Failed to read servers", *helpers::FAIL)
        };

        if let Some(server) = servers.get(server_name) {
            data = match Runner::connect(server_name.clone(), server.get(), false) {
                Some(mut remote) => (remote.process(*id).clone(), remote),
                None => crashln!("{} Failed to connect (name={server_name}, address={})", *helpers::FAIL, server.address),
            };
        } else {
            crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL)
        };

        let (item, remote) = data;
        let remote = remote.remote.unwrap();
        let info = http::info(&remote, *id);
        let path = item.path.to_string_lossy().into_owned();

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

        if let Ok(info) = info {
            let stats = info.json::<ItemSingle>().unwrap().stats;
            let children = if item.children.is_empty() { "none".to_string() } else { format!("{:?}", item.children) };

            let cpu_percent = match stats.cpu_percent {
                Some(percent) => format!("{percent:.2}%"),
                None => string!("0%"),
            };

            let memory_usage = match stats.memory_usage {
                Some(usage) => helpers::format_memory(usage.rss),
                None => string!("0b"),
            };

            let data = vec![Info {
                children,
                cpu_percent,
                memory_usage,
                id: string!(id),
                path: path.clone(),
                status: status.into(),
                restarts: item.restarts,
                name: item.name.clone(),
                pid: ternary!(item.running, format!("{pid}", pid = item.pid), string!("n/a")),
                log_out: format!("{}/{}-out.log", remote.config.log_path, item.name),
                log_error: format!("{}/{}-error.log", remote.config.log_path, item.name),
                hash: ternary!(item.watch.enabled, format!("{}  ", item.watch.hash), string!("none  ")),
                command: format!("{} {} '{}'", remote.config.shell, remote.config.args.join(" "), item.script),
                watch: ternary!(item.watch.enabled, format!("{path}/{}  ", item.watch.path), string!("disabled  ")),
                uptime: ternary!(item.running, format!("{}", helpers::format_duration(item.started)), string!("none")),
            }];

            render_info(data)
        }
    }
}

pub fn logs(id: &usize, lines: &usize, server_name: &String) {
    let mut runner: Runner = Runner::new();

    if !matches!(&**server_name, "internal" | "local") {
        let Some(servers) = config::servers().servers else {
            crashln!("{} Failed to read servers", *helpers::FAIL)
        };

        if let Some(server) = servers.get(server_name) {
            runner = match Runner::connect(server_name.clone(), server.get(), false) {
                Some(remote) => remote,
                None => crashln!("{} Failed to connect (name={server_name}, address={})", *helpers::FAIL, server.address),
            };
        } else {
            crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL)
        };

        let item = runner.info(*id).unwrap_or_else(|| crashln!("{} Process ({id}) not found", *helpers::FAIL));
        println!("{}", format!("Showing last {lines} lines for process [{id}] (change the value with --lines option)").yellow());

        for kind in vec!["error", "out"] {
            let logs = http::logs(&runner.remote.as_ref().unwrap(), *id, kind);

            if let Ok(log) = logs {
                if log.lines.is_empty() {
                    println!("{} No logs found for {}/{kind}", *helpers::FAIL, item.name);
                    continue;
                }

                file::logs_internal(log.lines, *lines, log.path, *id, kind, &item.name)
            }
        }
    } else {
        let item = runner.info(*id).unwrap_or_else(|| crashln!("{} Process ({id}) not found", *helpers::FAIL));
        println!("{}", format!("Showing last {lines} lines for process [{id}] (change the value with --lines option)").yellow());

        file::logs(item, *lines, "error");
        file::logs(item, *lines, "out");
    }
}

pub fn env(id: &usize, server_name: &String) {
    let mut runner: Runner = Runner::new();

    if !matches!(&**server_name, "internal" | "local") {
        let Some(servers) = config::servers().servers else {
            crashln!("{} Failed to read servers", *helpers::FAIL)
        };

        if let Some(server) = servers.get(server_name) {
            runner = match Runner::connect(server_name.clone(), server.get(), false) {
                Some(remote) => remote,
                None => crashln!("{} Failed to connect (name={server_name}, address={})", *helpers::FAIL, server.address),
            };
        } else {
            crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL)
        };
    }

    let item = runner.process(*id);
    item.env.iter().for_each(|(key, value)| println!("{}: {}", key, value.green()));
}

pub fn list(format: &String, server_name: &String) {
    let render_list = |runner: &mut Runner, internal: bool| {
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
                let mut cpu_percent: String = string!("0%");
                let mut memory_usage: String = string!("0b");

                if internal {
                    let mut usage_internals: (Option<f32>, Option<MemoryInfo>) = (None, None);

                    if let Ok(mut process) = Process::new(item.pid as u32) {
                        usage_internals = (process.cpu_percent().ok(), process.memory_info().ok());
                    }

                    cpu_percent = match usage_internals.0 {
                        Some(percent) => format!("{:.0}%", percent),
                        None => string!("0%"),
                    };

                    memory_usage = match usage_internals.1 {
                        Some(usage) => helpers::format_memory(usage.rss()),
                        None => string!("0b"),
                    };
                } else {
                    let info = http::info(&runner.remote.as_ref().unwrap(), id);

                    if let Ok(info) = info {
                        let stats = info.json::<ItemSingle>().unwrap().stats;

                        cpu_percent = match stats.cpu_percent {
                            Some(percent) => format!("{:.2}%", percent),
                            None => string!("0%"),
                        };

                        memory_usage = match stats.memory_usage {
                            Some(usage) => helpers::format_memory(usage.rss),
                            None => string!("0b"),
                        };
                    }
                }

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
                    status: status.into(),
                    cpu: format!("{cpu_percent}   "),
                    mem: format!("{memory_usage}   "),
                    id: id.to_string().cyan().bold().into(),
                    restarts: format!("{}  ", item.restarts),
                    name: format!("{}   ", item.name.clone()),
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
            match Runner::connect(server_name.clone(), server.get(), true) {
                Some(mut remote) => render_list(&mut remote, false),
                None => println!("{} Failed to fetch (name={server_name}, address={})", *helpers::FAIL, server.address),
            }
        } else {
            if matches!(&**server_name, "internal" | "all" | "global" | "local") {
                if *server_name == "all" || *server_name == "global" {
                    println!("{} Internal daemon", *helpers::SUCCESS);
                }
                render_list(&mut Runner::new(), true);
            } else {
                crashln!("{} Server '{server_name}' does not exist", *helpers::FAIL);
            }
        }

        if *server_name == "all" || *server_name == "global" {
            for (name, server) in servers {
                match Runner::connect(name.clone(), server.get(), true) {
                    Some(mut remote) => render_list(&mut remote, false),
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
        render_list(&mut Runner::new(), true);
    }
}
