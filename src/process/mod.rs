pub mod dump;
pub mod hash;
pub mod http;
pub mod id;
pub mod unix;

use crate::{
    config,
    config::structs::Server,
    file, helpers,
};

use std::{
    collections::HashSet, env, fs::File, path::PathBuf, sync::{Arc, Mutex}, thread, time::Duration,
};

use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use global_placeholders::global;
use macros_rs::{crashln, string, ternary, then};
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

impl From<unix::NativeMemoryInfo> for MemoryInfo {
    fn from(native: unix::NativeMemoryInfo) -> Self {
        MemoryInfo {
            rss: native.rss(),
            vms: native.vms(),
        }
    }
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

/// Process metadata
pub struct ProcessMetadata {
    /// Process name
    pub name: String,
    /// Shell command
    pub shell: String,
    /// Command
    pub command: String,
    /// Log path
    pub log_path: String,
    /// Arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: Vec<String>,
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
        match kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
            Ok(_) => {},
            Err(nix::errno::Errno::ESRCH) => {
                // Process already terminated
            },
            Err(err) => {
                log::error!("Failed to stop pid {}: {err:?}", pid);
            }
        }
    }
}

impl Runner {
    pub fn new() -> Self { dump::read() }

    pub fn refresh(&self) -> Self { Runner::new() }

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

            let pid = process_run(ProcessMetadata {
                args: config.args,
                name: name.clone(),
                shell: config.shell,
                command: command.clone(),
                log_path: config.log_path,
                env: unix::env(),
            }).unwrap_or_else(|err| crashln!("Failed to run process: {err}"));

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
            process_stop(process.pid).unwrap_or_else(|err| crashln!("Failed to stop process: {err}"));

            if let Err(err) = std::env::set_current_dir(&path) {
                process.running = false;
                process.children = vec![];
                process.crash.crashed = true;
                println!("{} Failed to set working directory {:?}\nError: {:#?}", *helpers::FAIL, path, err);
            } else {
                let mut temp_env = process.env.iter().map(|(key, value)| format!("{}={}", key, value)).collect::<Vec<String>>();
                temp_env.extend(unix::env());

                process.pid = process_run(ProcessMetadata {
                    args: config.args,
                    name: name.clone(),
                    shell: config.shell,
                    log_path: config.log_path,
                    command: script.to_string(),
                    env: temp_env,
                }).unwrap_or_else(|err| crashln!("Failed to run process: {err}"));

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

    pub fn try_info(&self, id: usize) -> &Process { self.list.get(&id).unwrap_or_else(|| crashln!("{} Process ({id}) not found", *helpers::FAIL)) }

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
            let process_to_stop = self.process(id);
            let pid_to_check = process_to_stop.pid;

            kill_children(process_to_stop.children.clone());
            let _ = process_stop(pid_to_check); // Continue even if stopping fails

            // waiting until Process is terminated
            for _ in 0..50 {
                match unix::NativeProcess::new(pid_to_check as u32) {
                    Ok(_p) => thread::sleep(Duration::from_millis(100)),
                    Err(_) => break,
                }
            }

            let process = self.process(id);
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

            if let Ok(process) = unix::NativeProcess::new(item.pid as u32) && 
                let Ok(mem_info_native) = process.memory_info() {
                cpu_percent = Some(get_process_cpu_usage_percentage(item.pid as i64));
                memory_usage = Some(MemoryInfo::from(mem_info_native));
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
            log::error!("{err}");
            crashln!("{} Failed to purge logs (path={})", *helpers::FAIL, self.error);
        }

        if let Err(err) = File::create(&self.error) {
            log::error!("{err}");
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

        if let Ok(process) = unix::NativeProcess::new(item.pid as u32) &&
            let Ok(mem_info_native) = process.memory_info() {
            cpu_percent = Some(get_process_cpu_usage_percentage(item.pid as i64));
            memory_usage = Some(MemoryInfo::from(mem_info_native));
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

/// Get the CPU usage percentage of the process
pub fn get_process_cpu_usage_percentage(pid: i64) -> f64 {
    match unix::NativeProcess::new(pid as u32) {
        Ok(process) => {
            match process.cpu_percent() {
                Ok(cpu_percent) => cpu_percent.min(100.0 * num_cpus::get() as f64),
                Err(_) => 0.0,
            }
        },
        Err(_) => 0.0,
    }
}

/// Stop the process
pub fn process_stop(pid: i64) -> Result<(), String> {
    let children = process_find_children(pid);
    
    // Stop child processes first
    for child_pid in children {
        let _ = kill(Pid::from_raw(child_pid as i32), Signal::SIGTERM);
        // Continue even if stopping child processes fails
    }
    
    // Stop parent process
    match kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
        Ok(_) => Ok(()),
        Err(nix::errno::Errno::ESRCH) => {
            // Process already terminated
            Ok(())
        },
        Err(err) => Err(format!("Failed to stop process {}: {:?}", pid, err))
    }
}

/// Find the children of the process
pub fn process_find_children(parent_pid: i64) -> Vec<i64> {
    let mut children = Vec::new();
    let mut to_check = vec![parent_pid];
    let mut checked = HashSet::new();

    #[cfg(target_os = "linux")]
    {
        while let Some(pid) = to_check.pop() {
            if checked.contains(&pid) {
                continue;
            }
            checked.insert(pid);

            let proc_path = format!("/proc/{}/task/{}/children", pid, pid);
            let Ok(contents) = std::fs::read_to_string(&proc_path) else {
                continue;
            };

            for child_pid_str in contents.split_whitespace() {
                if let Ok(child_pid) = child_pid_str.parse::<i64>() {
                    children.push(child_pid);
                    to_check.push(child_pid); // Check grandchildren
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        match unix::native_processes() {
            Ok(processes) => {
                // Build parent->children map in single pass
                let mut parent_map: HashMap<i64, Vec<i64>> = HashMap::new();

                processes.iter().for_each(|process| {
                    if let Ok(Some(ppid)) = process.ppid() {
                        parent_map.entry(ppid as i64)
                            .or_insert_with(Vec::new)
                            .push(process.pid() as i64);
                    }
                });

                while let Some(pid) = to_check.pop() &&
                    let Some(direct_children) = parent_map.get(&pid) {

                    for &child in direct_children {
                        if !checked.contains(&child) {
                            children.push(child);
                            to_check.push(child);
                            checked.insert(child);
                        }
                    }
                }
            },
            Err(_) => {
                log::warn!("Native process enumeration failed for PID {}", parent_pid);
            }
        }
    }

    children
}

/// Run the process
pub fn process_run(metadata: ProcessMetadata) -> Result<i64, String> {
    use std::process::{Command, Stdio};
    use std::fs::OpenOptions;

    let log_base = format!("{}/{}", metadata.log_path, metadata.name.replace(' ', "_"));
    let stdout_path = format!("{}-out.log", log_base);
    let stderr_path = format!("{}-error.log", log_base);

    // Create log files
    let stdout_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stdout_path)
        .map_err(|err| format!("Failed to open stdout log file {}: {:?}", stdout_path, err))?;

    let stderr_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_path)
        .map_err(|err| format!("Failed to open stderr log file {}: {:?}", stderr_path, err))?;

    // Execute process
    let mut cmd = Command::new(&metadata.shell);
    cmd.args(&metadata.args)
       .arg(&metadata.command)
       .envs(metadata.env.iter().map(|env_var| {
           let parts: Vec<&str> = env_var.splitn(2, '=').collect();
           if parts.len() == 2 {
               (parts[0], parts[1])
           } else {
               (env_var.as_str(), "")
           }
       }))
       .stdout(Stdio::from(stdout_file))
       .stderr(Stdio::from(stderr_file))
       .stdin(Stdio::null());

    let child = cmd.spawn()
        .map_err(|err| format!("Failed to spawn process: {:?}", err))?;

    let shell_pid = child.id() as i64;
    let actual_pid = unix::get_actual_child_pid(shell_pid);

    Ok(actual_pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    fn setup_test_runner() -> Runner {
        Runner {
            id: id::Id::new(1),
            list: BTreeMap::new(),
            remote: None,
        }
    }

    #[test]
    fn test_environment_variables() {
        let mut runner = setup_test_runner();
        let id = runner.id.next();
        
        let process = Process {
            id,
            pid: 12345,
            env: BTreeMap::new(),
            name: "test_process".to_string(),
            path: PathBuf::from("/tmp"),
            script: "echo 'hello world'".to_string(),
            restarts: 0,
            running: true,
            crash: Crash { crashed: false, value: 0 },
            watch: Watch {
                enabled: false,
                path: String::new(),
                hash: String::new(),
            },
            children: vec![],
            started: Utc::now(),
        };

        runner.list.insert(id, process);
        
        // Test setting environment variables
        let mut env = BTreeMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());
        env.insert("ANOTHER_VAR".to_string(), "another_value".to_string());
        
        runner.set_env(id, env);
        
        let process_env = &runner.info(id).unwrap().env;
        assert_eq!(process_env.get("TEST_VAR"), Some(&"test_value".to_string()));
        assert_eq!(process_env.get("ANOTHER_VAR"), Some(&"another_value".to_string()));
        
        // Test clearing environment variables
        runner.clear_env(id);
        assert!(runner.info(id).unwrap().env.is_empty());
    }

    #[test]
    fn test_children_processes() {
        let mut runner = setup_test_runner();
        let id = runner.id.next();
        
        let process = Process {
            id,
            pid: 12345,
            env: BTreeMap::new(),
            name: "test_process".to_string(),
            path: PathBuf::from("/tmp"),
            script: "echo 'hello world'".to_string(),
            restarts: 0,
            running: true,
            crash: Crash { crashed: false, value: 0 },
            watch: Watch {
                enabled: false,
                path: String::new(),
                hash: String::new(),
            },
            children: vec![],
            started: Utc::now(),
        };

        runner.list.insert(id, process);
        
        // Test setting children
        let children = vec![12346, 12347, 12348];
        runner.set_children(id, children.clone());
        
        assert_eq!(runner.info(id).unwrap().children, children);
    }

    #[test]
    fn test_cpu_usage_measurement() {
        // Test with current process (should return valid percentage)
        let current_pid = std::process::id() as i64;
        let cpu_usage = get_process_cpu_usage_percentage(current_pid);
        
        // CPU usage should be between 0 and 100 * number of cores
        assert!(cpu_usage >= 0.0);
        assert!(cpu_usage <= 100.0 * num_cpus::get() as f64);

        println!("CPU usage: {}", cpu_usage);
        
        // Test with invalid PID (should return 0.0)
        let invalid_pid = 999999;
        let cpu_usage = get_process_cpu_usage_percentage(invalid_pid);
        assert_eq!(cpu_usage, 0.0);
    }

    // Integration test for actual process operations
    #[test]
    #[ignore = "it requires actual process execution"]
    fn test_real_process_execution() {
        let metadata = ProcessMetadata {
            name: "test_echo".to_string(),
            shell: "/bin/sh".to_string(),
            command: "echo 'Hello from test'".to_string(),
            log_path: "/tmp".to_string(),
            args: vec!["-c".to_string()],
            env: vec!["TEST_ENV=test_value".to_string()],
        };

        match process_run(metadata) {
            Ok(pid) => {
                assert!(pid > 0);
                
                // Wait a bit for process to complete
                thread::sleep(Duration::from_millis(100));
                
                // Try to stop it (might already be finished)
                let _ = process_stop(pid);
            }
            Err(e) => {
                panic!("Failed to run test process: {}", e);
            }
        }
    }
}
