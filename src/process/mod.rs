mod dump;
mod log;

use crate::config;
use crate::file;
use crate::helpers::{self, Id};
use crate::service::{run, stop, ProcessMetadata};

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use macros_rs::{crashln, string};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Process {
    pub pid: i64,
    pub name: String,
    pub path: PathBuf,
    pub script: String,
    pub env: HashMap<String, String>,
    #[serde(with = "ts_milliseconds")]
    pub started: DateTime<Utc>,
    // pub restarts: u64,
    pub running: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Runner {
    pub id: Id,
    pub process_list: BTreeMap<String, Process>,
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

    pub fn start(&mut self, name: &String, command: &String) {
        let config = config::read().runner;
        let pid = run(ProcessMetadata {
            name: name.clone(),
            log_path: config.log_path,
            command: command.clone(),
            shell: config.shell,
            args: config.args,
        });

        self.process_list.insert(
            string!(self.id.next()),
            Process {
                pid,
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
            item.running = false;
            dump::write(&self);
        } else {
            crashln!("{} Process ({id}) not found", *helpers::FAIL);
        }
    }

    pub fn restart(&mut self, id: usize, name: &Option<String>) {
        if let Some(item) = self.info(id) {
            let script = item.script.clone();
            let path = item.path.clone();
            let env = item.env.clone();

            let name = match name {
                Some(name) => string!(name.trim()),
                None => string!(item.name.clone()),
            };

            if let Err(err) = std::env::set_current_dir(&path) {
                crashln!("{} Failed to set working directory {:?}\nError: {:#?}", *helpers::FAIL, path, err);
            };

            self.stop(id);

            let config = config::read().runner;
            let pid = run(ProcessMetadata {
                name: name.clone(),
                log_path: config.log_path,
                command: script.clone(),
                shell: config.shell,
                args: config.args,
            });

            self.process_list.insert(
                string!(id),
                Process {
                    pid,
                    env,
                    name,
                    path,
                    script,
                    running: true,
                    started: Utc::now(),
                },
            );
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

    pub fn set_id(&mut self, id: Id) {
        self.id = id;
        self.id.next();
        dump::write(&self);
    }

    pub fn info(&self, id: usize) -> Option<&Process> { self.process_list.get(&string!(id)) }
    pub fn list(&self) -> &BTreeMap<String, Process> { &self.process_list }
}
