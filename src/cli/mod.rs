mod args;
pub use args::*;

pub(crate) mod import;
pub(crate) mod internal;
pub(crate) mod server;

use internal::Internal;
use macros_rs::{crashln, string, ternary, then};
use pmc::{helpers, process::Runner};
use std::env;

pub(crate) fn format(server_name: &String) -> (String, String) {
    let kind = ternary!(matches!(&**server_name, "internal" | "local"), "", "remote ").to_string();
    return (kind, server_name.to_string());
}

pub fn get_version(short: bool) -> String {
    return match short {
        true => format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        false => match env!("GIT_HASH") {
            "" => format!("{} ({}) [{}]", env!("CARGO_PKG_VERSION"), env!("BUILD_DATE"), env!("PROFILE")),
            hash => format!("{} ({} {hash}) [{}]", env!("CARGO_PKG_VERSION"), env!("BUILD_DATE"), env!("PROFILE")),
        },
    };
}

pub fn start(name: &Option<String>, args: &Args, watch: &Option<String>, reset_env: &bool, server_name: &String) {
    let mut runner = Runner::new();
    let (kind, list_name) = format(server_name);

    let arg = match args.get_string() {
        Some(arg) => arg,
        None => "",
    };

    if arg == "all" {
        println!("{} Applying {kind}action startAllProcess", *helpers::SUCCESS);

        let largest = runner.size();
        match largest {
            Some(largest) => (0..*largest + 1).for_each(|id| {
                runner = Internal {
                    id,
                    server_name,
                    kind: kind.clone(),
                    runner: runner.clone(),
                }
                .restart(&None, &None, true);
            }),
            None => println!("{} Cannot start all, no processes found", *helpers::FAIL),
        }
    } else {
        match args {
            Args::Id(id) => {
                then!(*reset_env, runner.clear_env(*id));
                Internal { id: *id, runner, server_name, kind }.restart(name, watch, false);
            }
            Args::Script(script) => match runner.find(&script, server_name) {
                Some(id) => {
                    then!(*reset_env, runner.clear_env(id));
                    Internal { id, runner, server_name, kind }.restart(name, watch, false);
                }
                None => {
                    Internal { id: 0, runner, server_name, kind }.create(script, name, watch, false);
                }
            },
        }
    }

    Internal::list(&string!("default"), &list_name);
}

pub fn stop(item: &Item, server_name: &String) {
    let mut runner: Runner = Runner::new();
    let (kind, list_name) = format(server_name);

    let arg = match item.get_string() {
        Some(arg) => arg,
        None => "",
    };

    if arg == "all" {
        println!("{} Applying {kind}action stopAllProcess", *helpers::SUCCESS);

        let largest = runner.size();
        match largest {
            Some(largest) => (0..*largest + 1).for_each(|id| {
                runner = Internal {
                    id,
                    server_name,
                    kind: kind.clone(),
                    runner: runner.clone(),
                }
                .stop(true);
            }),
            None => println!("{} Cannot stop all, no processes found", *helpers::FAIL),
        }
    } else {
        match item {
            Item::Id(id) => {
                Internal { id: *id, runner, server_name, kind }.stop(false);
            }
            Item::Name(name) => match runner.find(&name, server_name) {
                Some(id) => {
                    Internal { id, runner, server_name, kind }.stop(false);
                }
                None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
            },
        }
    }

    Internal::list(&string!("default"), &list_name);
}

pub fn remove(item: &Item, server_name: &String) {
    let runner: Runner = Runner::new();
    let (kind, _) = format(server_name);

    match item {
        Item::Id(id) => Internal { id: *id, runner, server_name, kind }.remove(),
        Item::Name(name) => match runner.find(&name, server_name) {
            Some(id) => Internal { id, runner, server_name, kind }.remove(),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
    }
}

pub fn info(item: &Item, format: &String, server_name: &String) {
    let runner: Runner = Runner::new();
    let (kind, _) = self::format(server_name);

    match item {
        Item::Id(id) => Internal { id: *id, runner, server_name, kind }.info(format),
        Item::Name(name) => match runner.find(&name, server_name) {
            Some(id) => Internal { id, runner, server_name, kind }.info(format),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
    }
}

pub fn logs(item: &Item, lines: &usize, server_name: &String) {
    let runner: Runner = Runner::new();
    let (kind, _) = format(server_name);

    match item {
        Item::Id(id) => Internal { id: *id, runner, server_name, kind }.logs(lines),
        Item::Name(name) => match runner.find(&name, server_name) {
            Some(id) => Internal { id, runner, server_name, kind }.logs(lines),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
    }
}

// combine into a single function that handles multiple
pub fn env(item: &Item, server_name: &String) {
    let runner: Runner = Runner::new();
    let (kind, _) = format(server_name);

    match item {
        Item::Id(id) => Internal { id: *id, runner, server_name, kind }.env(),
        Item::Name(name) => match runner.find(&name, server_name) {
            Some(id) => Internal { id, runner, server_name, kind }.env(),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
    }
}

pub fn flush(item: &Item, server_name: &String) {
    let runner: Runner = Runner::new();
    let (kind, _) = format(server_name);

    match item {
        Item::Id(id) => Internal { id: *id, runner, server_name, kind }.flush(),
        Item::Name(name) => match runner.find(&name, server_name) {
            Some(id) => Internal { id, runner, server_name, kind }.flush(),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
    }
}
