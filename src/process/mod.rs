mod dump;
mod log;

use crate::{
    config, file, helpers,
    service::{run, stop, ProcessMetadata},
};

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use macros_rs::{crashln, string, ternary};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::{env, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Process {
    pub pid: i64,
    pub name: String,
    pub path: PathBuf,
    pub script: String,
    pub env: HashMap<String, String>,
    #[serde(with = "ts_milliseconds")]
    pub started: DateTime<Utc>,
    pub restarts: u64,
    pub running: bool,
    pub watch: Watch,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Watch {
    pub enabled: bool,
    pub path: String,
    pub hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Runner {
    pub id: id::Id,
    pub process_list: BTreeMap<String, Process>,
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
    pub fn new() -> Self {
        let dump = dump::read();

        let runner = Runner {
            id: dump.id,
            process_list: dump.process_list,
        };

        dump::write(&runner);
        return runner;
    }

    pub fn start(&mut self, name: &String, command: &String, watch: &Option<String>) {
        let config = config::read().runner;

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
        });

        self.process_list.insert(
            self.id.next().to_string(),
            Process {
                pid,
                watch,
                restarts: 0,
                running: true,
                path: file::cwd(),
                name: name.clone(),
                started: Utc::now(),
                script: command.clone(),
                env: env::vars().collect(),
            },
        );
        dump::write(&self);
    }

    pub fn stop(&mut self, id: usize) {
        if let Some(item) = self.process_list.get_mut(&string!(id)) {
            stop(item.pid);
            self.set_status(id, Status::Offline);
            dump::write(&self);
        } else {
            crashln!("{} Process ({id}) not found", *helpers::FAIL);
        }
    }

    pub fn restart(&mut self, id: usize, name: &Option<String>, watch: &Option<String>, dead: bool) {
        if let Some(item) = self.info(id) {
            let Process { path, script, .. } = item.clone();
            let restarts = ternary!(dead, item.restarts + 1, item.restarts);

            let watch = match watch {
                Some(watch) => Watch {
                    enabled: true,
                    path: string!(watch),
                    hash: hash::create(path.join(watch)),
                },
                None => Watch {
                    enabled: false,
                    path: string!(""),
                    hash: string!(""),
                },
            };

            let name = match name {
                Some(name) => string!(name.trim()),
                None => string!(item.name.clone()),
            };

            if let Err(err) = std::env::set_current_dir(&item.path) {
                crashln!("{} Failed to set working directory {:?}\nError: {:#?}", *helpers::FAIL, path, err);
            };

            self.stop(id);

            let config = config::read().runner;
            let pid = run(ProcessMetadata {
                command: script,
                args: config.args,
                name: name.clone(),
                shell: config.shell,
                log_path: config.log_path,
            });

            self.process_list.insert(string!(id), Process { pid, name, watch, restarts, ..item });
            self.set_status(id, Status::Running);
            self.set_started(id, Utc::now());

            dump::write(&self);
        } else {
            crashln!("{} Failed to restart process ({})", *helpers::FAIL, id);
        }
    }

    pub fn remove(&mut self, id: usize) {
        self.stop(id);
        self.process_list.remove(&string!(id));
        dump::write(&self);
    }

    pub fn set_id(&mut self, id: id::Id) {
        self.id = id;
        self.id.next();
        dump::write(&self);
    }

    pub fn set_status(&mut self, id: usize, status: Status) {
        if let Some(item) = self.process_list.get_mut(&string!(id)) {
            item.running = status.to_bool();
            dump::write(&self);
        } else {
            crashln!("{} Process ({id}) not found", *helpers::FAIL);
        }
    }

    pub fn set_started(&mut self, id: usize, time: DateTime<Utc>) {
        if let Some(item) = self.process_list.get_mut(&string!(id)) {
            item.started = time;
            dump::write(&self);
        } else {
            crashln!("{} Process ({id}) not found", *helpers::FAIL);
        }
    }

    pub fn info(&self, id: usize) -> Option<Process> { self.process_list.get(&string!(id)).cloned() }
    pub fn list(&self) -> &BTreeMap<String, Process> { &self.process_list }
}

pub mod hash;
pub mod id;
