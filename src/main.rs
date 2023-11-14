use macros_rs::{crashln, ternary};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{thread, time::Duration};

#[cxx::bridge]
pub mod process {
    unsafe extern "C++" {
        include!("pmc/src/include/process.h");
        include!("pmc/src/include/bridge.h");

        pub fn stop(pid: i64) -> i64;
        pub fn run(name: &str, log_path: &str, command: &str) -> i64;
    }
}

struct AutoIncrementId {
    counter: AtomicUsize,
}

impl AutoIncrementId {
    fn new() -> Self { AutoIncrementId { counter: AtomicUsize::new(0) } }
    fn next(&self) -> usize { self.counter.fetch_add(1, Ordering::SeqCst) }
}

#[derive(Clone, Debug)]
struct Process<'a> {
    pid: i64,
    name: &'a str,
    running: bool,
}

struct Runner<'a> {
    log_path: &'a str,
    id: AutoIncrementId,
    process_list: HashMap<usize, Process<'a>>,
}

impl<'a> Runner<'a> {
    fn new(path: &'a str) -> Self {
        Runner {
            log_path: path,
            id: AutoIncrementId::new(),
            process_list: HashMap::new(),
        }
    }

    fn start(&mut self, name: &'a str, command: &'a str) {
        let pid = process::run(&name, &self.log_path, &command);
        self.process_list.insert(self.id.next(), Process { pid, name, running: true });
    }

    fn stop(&mut self, id: usize) {
        if let Some(item) = self.process_list.get_mut(&id) {
            let pid = item.pid;
            process::stop(pid);
            item.running = false;
        } else {
            crashln!("process with {id} does not exist");
        }
    }

    fn info(&self, id: usize) -> Option<&Process> { self.process_list.get(&id) }
    fn list(&self) -> &HashMap<usize, Process<'a>> { &self.process_list }
}

fn main() {
    // save in .pmc/dump.toml
    // use global placeholders for home crate
    // use psutil for memory and cpu usage (in PAW)
    // create log dir if not exist
    // use clap cli and rataui for ui
    //    (pmc ls, pmc list, pmc ls --json, pmc list --json)
    //    [use clap command alias]

    let mut runner = Runner::new("tests/logs");

    runner.start("example", "node tests/index.js");
    println!("{:?}", runner.info(0));

    thread::sleep(Duration::from_millis(1000));

    runner.stop(0);
    println!("{:?}", runner.info(0));

    // runner.list().iter().for_each(|(id, item)| println!("id: {}\nname: {}", id, item.name));

    for (id, item) in runner.list() {
        println!("id: {id}\nname: {}\npid: {}\nstatus: {}", item.name, item.pid, ternary!(item.running, "online", "offline"));
    }
}
