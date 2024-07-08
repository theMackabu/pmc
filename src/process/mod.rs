mod unix;

use crate::{
    config,
    config::structs::Server,
    file, helpers,
    service::{run, stop, ProcessMetadata},
};

use std::{
    env,
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use global_placeholders::global;
use macros_rs::{crashln, string, ternary, then};
use psutil::process;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemSingle {
    pub info: Info,
    pub stats: Stats,
    pub watch: Watch,
    pub log: Log,
    pub raw: Raw,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Info {
    pub id: usize,
    pub pid: i64,
    pub name: String,
    pub status: String,
    #[schema(value_type = String, example = "/path")]
    pub path: PathBuf,
    pub uptime: String,
    pub command: String,
    pub children: Vec<i64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Stats {
    pub restarts: u64,
    pub start_time: i64,
    pub cpu_percent: Option<f64>,
    pub memory_usage: Option<MemoryInfo>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MemoryInfo {
    pub rss: u64,
    pub vms: u64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Log {
    pub out: String,
    pub error: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Raw {
    pub running: bool,
    pub crashed: bool,
    pub crashes: u64,
}

#[derive(Clone)]
pub struct LogInfo {
    pub out: String,
    pub error: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
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

#[derive(Clone)]
pub struct ProcessWrapper {
    pub id: usize,
    pub runner: Arc<Mutex<Runner>>,
}

pub type Env = BTreeMap<String, String>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Process {
    pub id: usize,
    pub pid: i64,
    pub env: Env,
    pub name: String,
    pub path: PathBuf,
    pub script: String,
    pub restarts: u64,
    pub running: bool,
    pub crash: Crash,
    pub watch: Watch,
    pub children: Vec<i64>,
    #[serde(with = "ts_milliseconds")]
    pub started: DateTime<Utc>,
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
    pub config: RemoteConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RemoteConfig {
    pub shell: String,
    pub args: Vec<String>,
    pub log_path: String,
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

macro_rules! lock {
    ($runner:expr) => {{
        match $runner.lock() {
            Ok(runner) => runner,
            Err(err) => crashln!("Unable to lock mutex: {err}"),
        }
    }};
}

fn kill_children(children: Vec<i64>) {
    for pid in children {
        if let Err(err) = kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
            log::error!("Failed to stop pid {pid}: {err:?}");
        };
    }
}

impl Runner {
    pub fn new() -> Self { dump::read() }

    pub fn connect(name: String, Server { address, token }: Server, verbose: bool) -> Option<Self> {
        let remote_config = match config::from(&address, token.as_deref()) {
            Ok(config) => config,
            Err(err) => {
                log::error!("{err}");
                return None;
            }
        };

        if let Ok(dump) = dump::from(&address, token.as_deref()) {
            then!(verbose, println!("{} Fetched remote (name={name}, address={address})", *helpers::SUCCESS));
            Some(Runner {
                remote: Some(Remote {
                    token,
                    address: string!(address),
                    config: remote_config,
                }),
                ..dump
            })
        } else {
            None
        }
    }

    pub fn start(&mut self, name: &String, command: &String, path: PathBuf, watch: &Option<String>) -> &mut Self {
        if let Some(remote) = &self.remote {
            if let Err(err) = http::create(remote, name, command, path, watch) {
                crashln!("{} Failed to start create {name}\nError: {:#?}", *helpers::FAIL, err);
            };
        } else {
            let id = self.id.next();
            let config = config::read().runner;
            let crash = Crash { crashed: false, value: 0 };

            let watch = match watch {
                Some(watch) => Watch {
                    enabled: true,
                    path: string!(watch),
                    hash: hash::create(file::cwd().join(watch)),
                },
                None => Watch {
                    enabled: false,
                    path: string!(""),
                    hash: string!(""),
                },
            };

            let pid = run(ProcessMetadata {
                args: config.args,
                name: name.clone(),
                shell: config.shell,
                command: command.clone(),
                log_path: config.log_path,
                env: unix::env(),
            });

            self.list.insert(
                id,
                Process {
                    id,
                    pid,
                    path,
                    watch,
                    crash,
                    restarts: 0,
                    running: true,
                    children: vec![],
                    name: name.clone(),
                    started: Utc::now(),
                    script: command.clone(),
                    env: env::vars().collect(),
                },
            );
        }

        return self;
    }

    pub fn restart(&mut self, id: usize, dead: bool) -> &mut Self {
        if let Some(remote) = &self.remote {
            if let Err(err) = http::restart(remote, id) {
                crashln!("{} Failed to start process {id}\nError: {:#?}", *helpers::FAIL, err);
            };
        } else {
            let process = self.process(id);
            let config = config::read().runner;
            let Process { path, script, name, .. } = process.clone();

            kill_children(process.children.clone());
            stop(process.pid);

            if let Err(err) = std::env::set_current_dir(&path) {
                process.running = false;
                process.children = vec![];
                process.crash.crashed = true;
                println!("{} Failed to set working directory {:?}\nError: {:#?}", *helpers::FAIL, path, err);
            } else {
                let mut temp_env = process.env.iter().map(|(key, value)| format!("{}={}", key, value)).collect::<Vec<String>>();
                temp_env.extend(unix::env());

                process.pid = run(ProcessMetadata {
                    args: config.args,
                    name: name.clone(),
                    shell: config.shell,
                    log_path: config.log_path,
                    command: script.to_string(),
                    env: temp_env,
                });

                process.running = true;
                process.children = vec![];
                process.started = Utc::now();
                process.crash.crashed = false;
                process.env.extend(env::vars().collect::<Env>());

                then!(dead, process.restarts += 1);
                then!(dead, process.crash.value += 1);
                then!(!dead, process.crash.value = 0);
            }
        }

        return self;
    }

    pub fn remove(&mut self, id: usize) {
        if let Some(remote) = &self.remote {
            if let Err(err) = http::remove(remote, id) {
                crashln!("{} Failed to stop remove {id}\nError: {:#?}", *helpers::FAIL, err);
            };
        } else {
            self.stop(id);
            self.list.remove(&id);
            self.save();
        }
    }

    pub fn set_id(&mut self, id: id::Id) {
        self.id = id;
        self.id.next();
        self.save();
    }

    pub fn set_status(&mut self, id: usize, status: Status) {
        self.process(id).running = status.to_bool();
        self.save();
    }

    pub fn items(&self) -> BTreeMap<usize, Process> { self.list.clone() }

    pub fn items_mut(&mut self) -> &mut BTreeMap<usize, Process> { &mut self.list }

    pub fn save(&self) { then!(self.remote.is_none(), dump::write(&self)) }

    pub fn count(&mut self) -> usize { self.list().count() }

    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    pub fn exists(&self, id: usize) -> bool { self.list.contains_key(&id) }

    pub fn info(&self, id: usize) -> Option<&Process> { self.list.get(&id) }

    pub fn size(&self) -> Option<&usize> { self.list.iter().map(|(k, _)| k).max() }

    pub fn list<'l>(&'l mut self) -> impl Iterator<Item = (&'l usize, &'l mut Process)> { self.list.iter_mut().map(|(k, v)| (k, v)) }

    pub fn process(&mut self, id: usize) -> &mut Process { self.list.get_mut(&id).unwrap_or_else(|| crashln!("{} Process ({id}) not found", *helpers::FAIL)) }

    pub fn pid(&self, id: usize) -> i64 { self.list.get(&id).unwrap_or_else(|| crashln!("{} Process ({id}) not found", *helpers::FAIL)).pid }

    pub fn get(self, id: usize) -> ProcessWrapper {
        ProcessWrapper {
            id,
            runner: Arc::new(Mutex::new(self)),
        }
    }

    pub fn set_crashed(&mut self, id: usize) -> &mut Self {
        self.process(id).crash.crashed = true;
        return self;
    }

    pub fn set_env(&mut self, id: usize, env: Env) -> &mut Self {
        self.process(id).env.extend(env);
        return self;
    }

    pub fn clear_env(&mut self, id: usize) -> &mut Self {
        if let Some(remote) = &self.remote {
            if let Err(err) = http::clear_env(remote, id) {
                crashln!("{} Failed to clear environment on {id}\nError: {:#?}", *helpers::FAIL, err);
            };
        } else {
            self.process(id).env = BTreeMap::new();
        }

        return self;
    }

    pub fn set_children(&mut self, id: usize, children: Vec<i64>) -> &mut Self {
        self.process(id).children = children;
        return self;
    }

    pub fn new_crash(&mut self, id: usize) -> &mut Self {
        self.process(id).crash.value += 1;
        return self;
    }

    pub fn stop(&mut self, id: usize) -> &mut Self {
        if let Some(remote) = &self.remote {
            if let Err(err) = http::stop(remote, id) {
                crashln!("{} Failed to stop process {id}\nError: {:#?}", *helpers::FAIL, err);
            };
        } else {
            let process = self.process(id);

            kill_children(process.children.clone());
            stop(process.pid);

            process.running = false;
            process.crash.crashed = false;
            process.crash.value = 0;
            process.children = vec![];
        }

        return self;
    }

    pub fn flush(&mut self, id: usize) -> &mut Self {
        if let Some(remote) = &self.remote {
            if let Err(err) = http::flush(remote, id) {
                crashln!("{} Failed to flush process {id}\nError: {:#?}", *helpers::FAIL, err);
            };
        } else {
            self.process(id).logs().flush();
        }

        return self;
    }

    pub fn rename(&mut self, id: usize, name: String) -> &mut Self {
        if let Some(remote) = &self.remote {
            if let Err(err) = http::rename(remote, id, name) {
                crashln!("{} Failed to rename process {id}\nError: {:#?}", *helpers::FAIL, err);
            };
        } else {
            self.process(id).name = name;
        }

        return self;
    }

    pub fn watch(&mut self, id: usize, path: &str, enabled: bool) -> &mut Self {
        let process = self.process(id);
        process.watch = Watch {
            enabled,
            path: string!(path),
            hash: ternary!(enabled, hash::create(process.path.join(path)), string!("")),
        };

        return self;
    }

    pub fn find(&self, name: &str, server_name: &String) -> Option<usize> {
        let mut runner = self.clone();

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

        runner.list.iter().find(|(_, p)| p.name == name).map(|(id, _)| *id)
    }

    pub fn fetch(&self) -> Vec<ProcessItem> {
        let mut processes: Vec<ProcessItem> = Vec::new();

        for (id, item) in self.items() {
            let mut memory_usage: Option<MemoryInfo> = None;
            let mut cpu_percent: Option<f64> = None;

            if let Ok(process) = process::Process::new(item.pid as u32) {
                let mem_info_psutil = process.memory_info().ok();

                cpu_percent = Some(super::service::get_process_cpu_usage_percentage(item.pid as i64));
                memory_usage = Some(MemoryInfo {
                    rss: mem_info_psutil.as_ref().unwrap().rss(),
                    vms: mem_info_psutil.as_ref().unwrap().vms(),
                });
            }

            let cpu_percent = match cpu_percent {
                Some(percent) => format!("{:.2}%", percent),
                None => string!("0.00%"),
            };

            let memory_usage = match memory_usage {
                Some(usage) => helpers::format_memory(usage.rss),
                None => string!("0b"),
            };

            let status = if item.running {
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

        return processes;
    }
}

impl LogInfo {
    pub fn flush(&self) {
        if let Err(err) = File::create(&self.out) {
            log::debug!("{err}");
            crashln!("{} Failed to purge logs (path={})", *helpers::FAIL, self.error);
        }

        if let Err(err) = File::create(&self.error) {
            log::debug!("{err}");
            crashln!("{} Failed to purge logs (path={})", *helpers::FAIL, self.error);
        }
    }
}

impl Process {
    /// Get a log paths of the process item
    pub fn logs(&self) -> LogInfo {
        let name = self.name.replace(" ", "_");

        LogInfo {
            out: global!("pmc.logs.out", name.as_str()),
            error: global!("pmc.logs.error", name.as_str()),
        }
    }
}

impl ProcessWrapper {
    /// Stop the process item
    pub fn stop(&mut self) { lock!(self.runner).stop(self.id).save(); }

    /// Restart the process item
    pub fn restart(&mut self) { lock!(self.runner).restart(self.id, false).save(); }

    /// Rename the process item
    pub fn rename(&mut self, name: String) { lock!(self.runner).rename(self.id, name).save(); }

    /// Enable watching a path on the process item
    pub fn watch(&mut self, path: &str) { lock!(self.runner).watch(self.id, path, true).save(); }

    /// Disable watching on the process item
    pub fn disable_watch(&mut self) { lock!(self.runner).watch(self.id, "", false).save(); }

    /// Set the process item as crashed
    pub fn crashed(&mut self) { lock!(self.runner).restart(self.id, true).save(); }

    /// Get the borrowed runner reference (lives till program end)
    pub fn get_runner(&mut self) -> &Runner { Box::leak(Box::new(lock!(self.runner))) }

    /// Append new environment values to the process item
    pub fn set_env(&mut self, env: Env) { lock!(self.runner).set_env(self.id, env).save(); }

    /// Clear environment values of the process item
    pub fn clear_env(&mut self) { lock!(self.runner).clear_env(self.id).save(); }

    /// Get a json dump of the process item
    pub fn fetch(&self) -> ItemSingle {
        let mut runner = lock!(self.runner);

        let item = runner.process(self.id);
        let config = config::read().runner;

        let mut memory_usage: Option<MemoryInfo> = None;
        let mut cpu_percent: Option<f64> = None;

        if let Ok(process) = process::Process::new(item.pid as u32) {
            let mem_info_psutil = process.memory_info().ok();

            cpu_percent = Some(super::service::get_process_cpu_usage_percentage(item.pid as i64));
            memory_usage = Some(MemoryInfo {
                rss: mem_info_psutil.as_ref().unwrap().rss(),
                vms: mem_info_psutil.as_ref().unwrap().vms(),
            });
        }

        let status = if item.running {
            string!("online")
        } else {
            match item.crash.crashed {
                true => string!("crashed"),
                false => string!("stopped"),
            }
        };

        ItemSingle {
            info: Info {
                status,
                id: item.id,
                pid: item.pid,
                name: item.name.clone(),
                path: item.path.clone(),
                children: item.children.clone(),
                uptime: helpers::format_duration(item.started),
                command: format!("{} {} '{}'", config.shell, config.args.join(" "), item.script.clone()),
            },
            stats: Stats {
                cpu_percent,
                memory_usage,
                restarts: item.restarts,
                start_time: item.started.timestamp_millis(),
            },
            watch: Watch {
                enabled: item.watch.enabled,
                hash: item.watch.hash.clone(),
                path: item.watch.path.clone(),
            },
            log: Log {
                out: item.logs().out,
                error: item.logs().error,
            },
            raw: Raw {
                running: item.running,
                crashed: item.crash.crashed,
                crashes: item.crash.value,
            },
        }
    }
}

pub mod dump;
pub mod hash;
pub mod http;
pub mod id;
