use crate::helpers::Id;
use crate::service::{run, stop};

use colored::Colorize;
use global_placeholders::global;
use macros_rs::{crashln, string};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::{fs, io::Result, path::Path, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Process {
    pub pid: i64,
    pub name: String,
    pub running: bool,
}

pub struct Runner {
    id: Id,
    log_path: String,
    process_list: HashMap<usize, Process>,
}

impl Runner {
    pub fn new(path: String) -> Self {
        Runner {
            log_path: path,
            // start at highest id in dump
            id: Id::new(0),
            process_list: HashMap::new(),
        }
    }

    pub fn start(&mut self, name: String, command: &String) {
        let pid = run(&name, &self.log_path, &command);
        self.process_list.insert(self.id.next(), Process { pid, name, running: true });
    }

    pub fn stop(&mut self, id: usize) {
        if let Some(item) = self.process_list.get_mut(&id) {
            let pid = item.pid;

            // add error catching in cc
            // to allow match Err() here
            stop(pid);
            item.running = false;
        } else {
            crashln!("Process with {id} does not exist");
        }
    }

    pub fn info(&self, id: usize) -> Option<&Process> { self.process_list.get(&id) }
    pub fn list(&self) -> &HashMap<usize, Process> { &self.process_list }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DumpFile {
    pub process_list: BTreeMap<usize, Process>,
}

fn read_dump() -> DumpFile {
    let contents = match fs::read_to_string(global!("pmc.dump")) {
        Ok(contents) => contents,
        Err(err) => crashln!("Cannot find dumpfile.\n{}", string!(err).white()),
    };

    match toml::from_str(&contents).map_err(|err| string!(err)) {
        Ok(parsed) => parsed,
        Err(err) => crashln!("Cannot read dumpfile.\n{}", err.white()),
    }
}

fn write_dump(dump: &DumpFile) {
    let contents = match toml::to_string(dump) {
        Ok(contents) => contents,
        Err(err) => crashln!("Cannot parse dump.\n{}", string!(err).white()),
    };

    if let Err(err) = fs::write(global!("pmc.dump"), contents) {
        crashln!("Error writing dumpfile.\n{}", string!(err).white())
    }
}
