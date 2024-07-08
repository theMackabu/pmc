use colored::Colorize;
use macros_rs::{crashln, string, ternary, then};
use psutil::process::{MemoryInfo, Process};
use regex::Regex;
use serde::Serialize;
use serde_json::json;

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

pub struct Internal<'i> {
    pub id: usize,
    pub runner: Runner,
    pub kind: String,
    pub server_name: &'i str,
}

impl<'i> Internal<'i> {
    pub fn create(mut self, script: &String, name: &Option<String>, watch: &Option<String>, silent: bool) -> Runner {
        let config = config::read();
        let name = match name {
            Some(name) => string!(name),
            None => string!(script.split_whitespace().next().unwrap_or_default()),
        };

        if matches!(self.server_name, "internal" | "local") {
            let pattern = Regex::new(r"(?m)^[a-zA-Z0-9]+(/[a-zA-Z0-9]+)*(\.js|\.ts)?$").unwrap();

            if pattern.is_match(script) {
                let script = format!("{} {script}", config.runner.node);
                self.runner.start(&name, &script, file::cwd(), watch).save();
            } else {
                self.runner.start(&name, script, file::cwd(), watch).save();
            }
        } else {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                match Runner::connect(self.server_name.into(), server.get(), false) {
                    Some(mut remote) => remote.start(&name, script, file::cwd(), watch),
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name,)
            };
        }

        then!(!silent, println!("{} Creating {}process with ({name})", *helpers::SUCCESS, self.kind));
        then!(!silent, println!("{} {}Created ({name}) ✓", *helpers::SUCCESS, self.kind));

        return self.runner;
    }

    pub fn restart(mut self, name: &Option<String>, watch: &Option<String>, reset_env: bool, silent: bool) -> Runner {
        then!(!silent, println!("{} Applying {}action restartProcess on ({})", *helpers::SUCCESS, self.kind, self.id));

        if matches!(self.server_name, "internal" | "local") {
            let mut item = self.runner.get(self.id);

            match watch {
                Some(path) => item.watch(path),
                None => item.disable_watch(),
            }

            then!(reset_env, item.clear_env());

            name.as_ref().map(|n| item.rename(n.trim().replace("\n", "")));
            item.restart();

            self.runner = item.get_runner().clone();
        } else {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                match Runner::connect(self.server_name.into(), server.get(), false) {
                    Some(remote) => {
                        let mut item = remote.get(self.id);

                        then!(reset_env, item.clear_env());

                        name.as_ref().map(|n| item.rename(n.trim().replace("\n", "")));
                        item.restart();
                    }
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                }
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };
        }

        if !silent {
            println!("{} Restarted {}({}) ✓", *helpers::SUCCESS, self.kind, self.id);
            log!("process started (id={})", self.id);
        }

        return self.runner;
    }

    pub fn stop(mut self, silent: bool) -> Runner {
        then!(!silent, println!("{} Applying {}action stopProcess on ({})", *helpers::SUCCESS, self.kind, self.id));

        if !matches!(self.server_name, "internal" | "local") {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                self.runner = match Runner::connect(self.server_name.into(), server.get(), false) {
                    Some(remote) => remote,
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };
        }

        let mut item = self.runner.get(self.id);
        item.stop();
        self.runner = item.get_runner().clone();

        if !silent {
            println!("{} Stopped {}({}) ✓", *helpers::SUCCESS, self.kind, self.id);
            log!("process stopped {}(id={})", self.kind, self.id);
        }

        return self.runner;
    }

    pub fn remove(mut self) {
        println!("{} Applying {}action removeProcess on ({})", *helpers::SUCCESS, self.kind, self.id);

        if !matches!(self.server_name, "internal" | "local") {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                self.runner = match Runner::connect(self.server_name.into(), server.get(), false) {
                    Some(remote) => remote,
                    None => crashln!("{} Failed to remove (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };
        }

        self.runner.remove(self.id);
        println!("{} Removed {}({}) ✓", *helpers::SUCCESS, self.kind, self.id);
        log!("process removed (id={})", self.id);
    }

    pub fn flush(&mut self) {
        println!("{} Applying {}action flushLogs on ({})", *helpers::SUCCESS, self.kind, self.id);

        if !matches!(self.server_name, "internal" | "local") {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                self.runner = match Runner::connect(self.server_name.into(), server.get(), false) {
                    Some(remote) => remote,
                    None => crashln!("{} Failed to remove (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };
        }

        self.runner.flush(self.id);
        println!("{} Flushed Logs {}({}) ✓", *helpers::SUCCESS, self.kind, self.id);
        log!("process logs cleaned (id={})", self.id);
    }

    pub fn info(&self, format: &String) {
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
                        println!("{}\n{table}\n", format!("Describing {}process with id ({})", self.kind, self.id).on_bright_white().black());
                        println!(" {}", format!("Use `pmc logs {} [--lines <num>]` to display logs", self.id).white());
                        println!(" {}", format!("Use `pmc env {}`  to display environment variables", self.id).white());
                    }
                };
            };
        };

        if matches!(self.server_name, "internal" | "local") {
            if let Some(home) = home::home_dir() {
                let config = config::read().runner;
                let mut runner = Runner::new();
                let item = runner.process(self.id);

                let mut memory_usage: Option<MemoryInfo> = None;
                let mut cpu_percent: Option<f64> = None;

                let path = file::make_relative(&item.path, &home).to_string_lossy().into_owned();
                let children = if item.children.is_empty() { "none".to_string() } else { format!("{:?}", item.children) };

                if let Ok(process) = Process::new(item.pid as u32) {
                    memory_usage = process.memory_info().ok();
                    cpu_percent = Some(pmc::service::get_process_cpu_usage_percentage(item.pid as i64));
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
                    id: string!(self.id),
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

            if let Some(server) = servers.get(self.server_name) {
                data = match Runner::connect(self.server_name.into(), server.get(), false) {
                    Some(mut remote) => (remote.process(self.id).clone(), remote),
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };

            let (item, remote) = data;
            let remote = remote.remote.unwrap();
            let info = http::info(&remote, self.id);
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
                    id: string!(self.id),
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

    pub fn logs(mut self, lines: &usize) {
        if !matches!(self.server_name, "internal" | "local") {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                self.runner = match Runner::connect(self.server_name.into(), server.get(), false) {
                    Some(remote) => remote,
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };

            let item = self.runner.info(self.id).unwrap_or_else(|| crashln!("{} Process ({}) not found", *helpers::FAIL, self.id));
            println!(
                "{}",
                format!("Showing last {lines} lines for {}process [{}] (change the value with --lines option)", self.kind, self.id).yellow()
            );

            for kind in vec!["error", "out"] {
                let logs = http::logs(&self.runner.remote.as_ref().unwrap(), self.id, kind);

                if let Ok(log) = logs {
                    if log.lines.is_empty() {
                        println!("{} No logs found for {}/{kind}", *helpers::FAIL, item.name);
                        continue;
                    }

                    file::logs_internal(log.lines, *lines, log.path, self.id, kind, &item.name)
                }
            }
        } else {
            let item = self.runner.info(self.id).unwrap_or_else(|| crashln!("{} Process ({}) not found", *helpers::FAIL, self.id));
            println!(
                "{}",
                format!("Showing last {lines} lines for {}process [{}] (change the value with --lines option)", self.kind, self.id).yellow()
            );

            file::logs(item, *lines, "error");
            file::logs(item, *lines, "out");
        }
    }

    pub fn env(mut self) {
        println!("{}", format!("Showing env for {}process {}:\n", self.kind, self.id).bright_yellow());

        if !matches!(self.server_name, "internal" | "local") {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                self.runner = match Runner::connect(self.server_name.into(), server.get(), false) {
                    Some(remote) => remote,
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };
        }

        let item = self.runner.process(self.id);
        item.env.iter().for_each(|(key, value)| println!("{}: {}", key, value.green()));
    }

    pub fn save(server_name: &String) {
        if !matches!(&**server_name, "internal" | "local") {
            crashln!("{} Cannot force save on remote servers", *helpers::FAIL)
        }

        println!("{} Saved current processes to dumpfile", *helpers::SUCCESS);
        Runner::new().save();
    }

    pub fn restore(server_name: &String) {
        let mut runner = Runner::new();
        let (kind, list_name) = super::format(server_name);

        if !matches!(&**server_name, "internal" | "local") {
            crashln!("{} Cannot restore on remote servers", *helpers::FAIL)
        }

        Runner::new().list().for_each(|(id, p)| {
            if p.running == true {
                runner = Internal {
                    id: *id,
                    server_name,
                    kind: kind.clone(),
                    runner: runner.clone(),
                }
                .restart(&None, &None, false, true);
            }
        });

        println!("{} Restored process statuses from dumpfile", *helpers::SUCCESS);
        Internal::list(&string!("default"), &list_name);
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
                        let mut usage_internals: (Option<f64>, Option<MemoryInfo>) = (None, None);

                        if let Ok(process) = Process::new(item.pid as u32) {
                            usage_internals = (Some(pmc::service::get_process_cpu_usage_percentage(item.pid as i64)), process.memory_info().ok());
                        }

                        cpu_percent = match usage_internals.0 {
                            Some(percent) => format!("{:.2}%", percent),
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
}
