use crate::id::AutoIncrement;
use crate::service::{run, stop};
use macros_rs::crashln;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Process<'a> {
    pub pid: i64,
    pub name: &'a str,
    pub running: bool,
}

pub struct Runner<'a> {
    id: AutoIncrement,
    log_path: &'a str,
    process_list: HashMap<usize, Process<'a>>,
}

impl<'a> Runner<'a> {
    pub fn new(path: &'a str) -> Self {
        Runner {
            log_path: path,
            // start at highest id in dump
            id: AutoIncrement::new(0),
            process_list: HashMap::new(),
        }
    }

    pub fn start(&mut self, name: &'a str, command: &'a str) {
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
            crashln!("process with {id} does not exist");
        }
    }

    pub fn info(&self, id: usize) -> Option<&Process> { self.process_list.get(&id) }
    pub fn list(&self) -> &HashMap<usize, Process<'a>> { &self.process_list }
}
