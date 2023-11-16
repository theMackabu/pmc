mod dump;

use crate::file;
use crate::helpers::{self, Id};
use crate::service::{run, stop};

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
    pub running: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Runner {
    id: Id,
    log_path: String,
    process_list: BTreeMap<String, Process>,
}

impl Runner {
    pub fn new() -> Self {
        let dump = dump::read();

        let runner = Runner {
            id: dump.id,
            log_path: dump.log_path,
            process_list: dump.process_list,
        };

        dump::write(&runner);
        return runner;
    }

    pub fn start(&mut self, name: String, command: &String) {
        let pid = run(&name, &self.log_path, &command);
        self.process_list.insert(
            string!(self.id.next()),
            Process {
                pid,
                name,
                env: env::vars().collect(),
                path: file::cwd(),
                started: Utc::now(),
                script: string!(command),
                running: true,
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
            let pid = run(&name, &self.log_path, &script);

            self.process_list.insert(
                string!(id),
                Process {
                    pid,
                    name,
                    path,
                    env,
                    script,
                    started: Utc::now(),
                    running: true,
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

    pub fn info(&self, id: usize) -> Option<&Process> { self.process_list.get(&string!(id)) }
    pub fn list(&self) -> &BTreeMap<String, Process> { &self.process_list }
}
