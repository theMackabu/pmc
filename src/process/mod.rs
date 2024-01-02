mod http;

use crate::{
    config,
    config::structs::Server,
    file, helpers,
    service::{run, stop, ProcessMetadata},
};

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use global_placeholders::global;
use macros_rs::{crashln, string, ternary, then};
use psutil::process::{self, MemoryInfo};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::{env, path::PathBuf};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ItemSingle {
    info: Info,
    stats: Stats,
    watch: Watch,
    log: Log,
    raw: Raw,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    id: usize,
    pid: i64,
    name: String,
    status: String,
    #[schema(value_type = String, example = "/path")]
    path: PathBuf,
    uptime: String,
    command: String,
}

#[derive(Serialize, ToSchema)]
pub struct Stats {
    restarts: u64,
    start_time: i64,
    cpu_percent: Option<f32>,
    memory_usage: Option<MemoryInfo>,
}

#[derive(Serialize, ToSchema)]
pub struct Log {
    out: String,
    error: String,
}

#[derive(Serialize, ToSchema)]
pub struct Raw {
    running: bool,
    crashed: bool,
    crashes: u64,
}

#[derive(Serialize, ToSchema)]
pub struct ProcessItem {
    pid: i64,
    id: usize,
    cpu: String,
    mem: String,
    name: String,
    restarts: u64,
    status: String,
    uptime: String,
    #[schema(example = "/path")]
    watch_path: String,
    #[schema(value_type = String, example = "2000-01-01T01:00:00.000Z")]
    start_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Process {
    pub id: usize,
    pub pid: i64,
    pub name: String,
    pub path: PathBuf,
    pub script: String,
    pub env: HashMap<String, String>,
    #[serde(with = "ts_milliseconds")]
    pub started: DateTime<Utc>,
    pub restarts: u64,
    pub running: bool,
    pub crash: Crash,
    pub watch: Watch,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crash {
    pub crashed: bool,
    pub value: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct Watch {
    pub enabled: bool,
    #[schema(example = "/path")]
    pub path: String,
    pub hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Runner {
    pub id: id::Id,
    #[serde(skip)]
    pub remote: Option<Remote>,
    pub list: BTreeMap<usize, Process>,
}

#[derive(Clone, Debug)]
pub struct Remote {
    address: String,
    token: Option<String>,
}

pub enum Status {
    Offline,
    Running,
}

impl Status {
    pub fn to_bool(&self) -> bool {
        match self {
            Status::Offline => false,
            Status::Running => true,
        }
    }
}

impl Runner {
    pub fn new() -> Self { dump::read() }

    pub fn connect(name: String, server: Server) -> Option<Self> {
        let Server { address, token } = server;

        match dump::from(&address, token.as_deref()) {
            Ok(dump) => {
                println!("{} Fetched remote (name={name}, address={address})", *helpers::SUCCESS);
                return Some(Runner {
                    remote: Some(Remote { token, address: string!(address) }),
                    ..dump
                });
            }
            Err(err) => {
                log::debug!("{err}");
                return None;
            }
        }
    }

    pub fn start(&mut self, name: &String, command: &String, watch: &Option<String>) -> &mut Self {
        let id = self.id.next();
        let config = config::read().runner;
        let crash = Crash { crashed: false, value: 0 };

        let watch = match watch {
            Some(watch) => Watch {
                enabled: true,
                path: string!(watch),
                hash: hash::create(file::cwd().join(watch)),
            },
            None => {
                Watch {
                    enabled: false,
                    path: string!(""),
                    hash: string!(""),
                }
            }
        };

        let pid = run(ProcessMetadata {
            args: config.args,
            name: name.clone(),
            shell: config.shell,
            command: command.clone(),
            log_path: config.log_path,
        });

        self.list.insert(
            id,
            Process {
                id,
                pid,
                watch,
                crash,
                restarts: 0,
                running: true,
                path: file::cwd(),
                name: name.clone(),
                started: Utc::now(),
                script: command.clone(),
                env: env::vars().collect(),
            },
        );

        return self;
    }

    pub fn restart(&mut self, id: usize, dead: bool) -> &mut Self {
        let item = self.get(id);
        let Process { path, script, name, .. } = item.clone();

        if let Err(err) = std::env::set_current_dir(&item.path) {
            crashln!("{} Failed to set working directory {:?}\nError: {:#?}", *helpers::FAIL, path, err);
        };

        item.stop();

        let config = config::read().runner;

        item.crash.crashed = false;
        item.pid = run(ProcessMetadata {
            command: script,
            args: config.args,
            name: name.clone(),
            shell: config.shell,
            log_path: config.log_path,
        });

        item.running = true;
        item.started = Utc::now();
        then!(dead, item.restarts += 1);

        return self;
    }

    pub fn remove(&mut self, id: usize) {
        self.stop(id);
        self.list.remove(&id);
        dump::write(&self);
    }

    pub fn set_id(&mut self, id: id::Id) {
        self.id = id;
        self.id.next();
        dump::write(&self);
    }

    pub fn set_status(&mut self, id: usize, status: Status) {
        self.get(id).running = status.to_bool();
        dump::write(&self);
    }

    pub fn items(&mut self) -> BTreeMap<usize, Process> { self.list.clone() }
    pub fn items_mut(&mut self) -> &mut BTreeMap<usize, Process> { &mut self.list }

    pub fn save(&self) { then!(self.remote.is_none(), dump::write(&self)) }
    pub fn count(&mut self) -> usize { self.list().count() }
    pub fn is_empty(&self) -> bool { self.list.is_empty() }
    pub fn exists(&mut self, id: usize) -> bool { self.list.contains_key(&id) }
    pub fn info(&mut self, id: usize) -> Option<&Process> { self.list.get(&id) }
    pub fn list<'l>(&'l mut self) -> impl Iterator<Item = (&'l usize, &'l mut Process)> { self.list.iter_mut().map(|(k, v)| (k, v)) }
    pub fn get(&mut self, id: usize) -> &mut Process { self.list.get_mut(&id).unwrap_or_else(|| crashln!("{} Process ({id}) not found", *helpers::FAIL)) }

    pub fn set_crashed(&mut self, id: usize) -> &mut Self {
        self.get(id).crash.crashed = true;
        return self;
    }

    pub fn new_crash(&mut self, id: usize) -> &mut Self {
        self.get(id).crash.value += 1;
        return self;
    }

    pub fn stop(&mut self, id: usize) -> &mut Self {
        let item = self.get(id);
        stop(item.pid);

        item.running = false;
        item.crash.crashed = false;
        item.crash.value = 0;

        return self;
    }

    pub fn rename(&mut self, id: usize, name: String) -> &mut Self {
        self.get(id).name = name;
        return self;
    }

    pub fn watch(&mut self, id: usize, path: &str, enabled: bool) -> &mut Self {
        let item = self.get(id);
        item.watch = Watch {
            enabled,
            path: string!(path),
            hash: ternary!(enabled, hash::create(item.path.join(path)), string!("")),
        };

        return self;
    }

    pub fn json(&mut self) -> Value {
        let mut processes: Vec<ProcessItem> = Vec::new();

        for (id, item) in self.items() {
            let mut memory_usage: Option<MemoryInfo> = None;
            let mut cpu_percent: Option<f32> = None;

            if let Ok(mut process) = process::Process::new(item.pid as u32) {
                memory_usage = process.memory_info().ok();
                cpu_percent = process.cpu_percent().ok();
            }

            let cpu_percent = match cpu_percent {
                Some(percent) => format!("{:.2}%", percent),
                None => string!("0.00%"),
            };

            let memory_usage = match memory_usage {
                Some(usage) => helpers::format_memory(usage.rss()),
                None => string!("0b"),
            };

            let status =
                if item.running {
                    string!("online")
                } else {
                    match item.crash.crashed {
                        true => string!("crashed"),
                        false => string!("stopped"),
                    }
                };

            processes.push(ProcessItem {
                id,
                status,
                pid: item.pid,
                cpu: cpu_percent,
                mem: memory_usage,
                restarts: item.restarts,
                name: item.name.clone(),
                start_time: item.started,
                watch_path: item.watch.path.clone(),
                uptime: helpers::format_duration(item.started),
            });
        }

        json!(processes)
    }
}

impl Process {
    pub fn stop(&mut self) { Runner::new().stop(self.id).save(); }
    pub fn watch(&mut self, path: &str) { Runner::new().watch(self.id, path, true).save(); }
    pub fn disable_watch(&mut self) { Runner::new().watch(self.id, "", false).save(); }
    pub fn rename(&mut self, name: String) { Runner::new().rename(self.id, name).save(); }
    pub fn restart(&mut self) { Runner::new().restart(self.id, false).save(); }

    pub fn crashed(&mut self) -> &mut Process {
        Runner::new().new_crash(self.id).save();
        Runner::new().restart(self.id, true).save();
        return self;
    }

    pub fn json(&mut self) -> Value {
        let config = config::read().runner;

        let mut memory_usage: Option<MemoryInfo> = None;
        let mut cpu_percent: Option<f32> = None;

        if let Ok(mut process) = process::Process::new(self.pid as u32) {
            memory_usage = process.memory_info().ok();
            cpu_percent = process.cpu_percent().ok();
        }

        let status = if self.running {
            string!("online")
        } else {
            match self.crash.crashed {
                true => string!("crashed"),
                false => string!("stopped"),
            }
        };

        json!(ItemSingle {
            info: Info {
                status,
                id: self.id,
                pid: self.pid,
                name: self.name.clone(),
                path: self.path.clone(),
                uptime: helpers::format_duration(self.started),
                command: format!("{} {} '{}'", config.shell, config.args.join(" "), self.script.clone()),
            },
            stats: Stats {
                cpu_percent,
                memory_usage,
                restarts: self.restarts,
                start_time: self.started.timestamp_millis(),
            },
            watch: Watch {
                enabled: self.watch.enabled,
                hash: self.watch.hash.clone(),
                path: self.watch.path.clone(),
            },
            log: Log {
                out: global!("pmc.logs.out", self.name.as_str()),
                error: global!("pmc.logs.error", self.name.as_str()),
            },
            raw: Raw {
                running: self.running,
                crashed: self.crash.crashed,
                crashes: self.crash.value,
            }
        })
    }
}

pub mod dump;
pub mod hash;
pub mod id;
