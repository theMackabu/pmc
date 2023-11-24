use crate::{
    config, file, helpers,
    service::{run, stop, ProcessMetadata},
};

use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use macros_rs::{clone, crashln, string, then};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::{env, path::PathBuf};

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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Watch {
    pub enabled: bool,
    pub path: String,
    pub hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Runner {
    pub id: id::Id,
    pub list: BTreeMap<usize, Process>,
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
        let runner = Runner { id: dump.id, list: dump.list };

        dump::write(&runner);
        return runner;
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
        dump::write(&self);

        return self;
    }

    pub fn restart(&mut self, id: usize, name: String, dead: bool) -> &mut Self {
        let item = self.get(id);
        let Process { path, script, .. } = item.clone();

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

        item.watch = Watch {
            enabled: false,
            path: string!(""),
            hash: string!(""),
        };

        item.name = name;
        item.running = true;
        item.started = Utc::now();
        then!(dead, item.restarts += 1);

        // assign!(item, {name, pid, watch});

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
        let item = self.get(id);
        item.running = status.to_bool();
        dump::write(&self);
    }

    pub fn save(&self) { dump::write(&self); }
    pub fn count(&mut self) -> usize { self.list().count() }
    pub fn is_empty(&self) -> bool { self.list.is_empty() }
    pub fn items(&mut self) -> &mut BTreeMap<usize, Process> { &mut self.list }
    pub fn list<'a>(&'a mut self) -> impl Iterator<Item = (&'a usize, &'a mut Process)> { self.list.iter_mut().map(|(k, v)| (k, v)) }
    pub fn get(&mut self, id: usize) -> &mut Process { self.list.get_mut(&id).unwrap_or_else(|| crashln!("{} Process ({id}) not found", *helpers::FAIL)) }

    pub fn set_crashed(&mut self, id: usize) -> &mut Self {
        let item = self.get(id);
        item.crash.crashed = true;
        return self;
    }

    pub fn new_crash(&mut self, id: usize) -> &mut Self {
        let item = self.get(id);
        item.crash.value += 1;
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
        let item = self.get(id);
        item.name = name;
        return self;
    }

    pub fn watch(&mut self, id: usize, path: String) -> &mut Self {
        let item = self.get(id);
        item.watch = Watch {
            enabled: true,
            path: clone!(path),
            hash: hash::create(item.path.join(path)),
        };

        return self;
    }
}

impl Process {
    pub fn stop(&mut self) { Runner::new().stop(self.id).save(); }
    pub fn watch(&mut self, path: String) { Runner::new().watch(self.id, path).save(); }
    pub fn rename(&mut self, name: String) { Runner::new().rename(self.id, name).save(); }

    pub fn restart(&mut self) -> &mut Process {
        Runner::new().restart(self.id, clone!(self.name), false).save();
        return self;
    }

    pub fn crashed(&mut self) -> &mut Process {
        Runner::new().new_crash(self.id).save();
        Runner::new().restart(self.id, clone!(self.name), true).save();
        return self;
    }
}

pub mod dump;
pub mod hash;
pub mod id;
