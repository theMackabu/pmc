mod dump;

use crate::helpers::Id;
use crate::service::{run, stop};

use macros_rs::{crashln, string};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Process {
    pub pid: i64,
    pub name: String,
    pub running: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Runner {
    id: Id,
    log_path: String,
    process_list: BTreeMap<String, Process>,
}

impl Runner {
    pub fn new(path: String) -> Self {
        let dump = dump::read();

        let runner = Runner {
            log_path: path,
            id: dump.id,
            process_list: dump.process_list,
        };

        dump::write(&runner);
        return runner;
    }

    pub fn start(&mut self, name: String, command: &String) {
        let pid = run(&name, &self.log_path, &command);
        self.process_list.insert(string!(self.id.next()), Process { pid, name, running: true });
        dump::write(&self);
    }

    pub fn stop(&mut self, id: usize) {
        if let Some(item) = self.process_list.get_mut(&string!(id)) {
            let pid = item.pid;

            // add error catching in cc
            // to allow match Err() here
            stop(pid);
            item.running = false;
            dump::write(&self);
        } else {
            crashln!("Process with {id} does not exist");
        }
    }

    pub fn info(&self, id: usize) -> Option<&Process> { self.process_list.get(&string!(id)) }
    pub fn list(&self) -> &BTreeMap<String, Process> { &self.process_list }
}
