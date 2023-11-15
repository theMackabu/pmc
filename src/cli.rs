use crate::process::Runner;
use crate::structs::Args;

use global_placeholders::global;
use macros_rs::string;
use std::env;

pub fn get_version(short: bool) -> String {
    return match short {
        true => format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        false => format!("{} ({} {}) [{}]", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"), env!("BUILD_DATE"), env!("PROFILE")),
    };
}

pub fn start(name: &Option<String>, args: &Option<Args>) {
    let mut runner = Runner::new(global!("pmc.logs"));

    let name = match name {
        Some(name) => string!(name),
        None => string!(""),
    };

    match args {
        Some(Args::Id(id)) => println!("{}", id),
        Some(Args::Script(script)) => runner.start(name, script),
        None => {}
    }
}

pub fn stop(id: &usize) {
    let mut runner = Runner::new(global!("pmc.logs"));
    runner.stop(*id);
    println!("Stopped process");
}

pub fn list() {
    let runner = Runner::new(global!("pmc.logs"));
    for (id, item) in runner.list() {
        println!("id: {id}\nname: {}\npid: {}\nstatus: {}", item.name, item.pid, item.running);
    }
}
