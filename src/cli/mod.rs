mod args;
pub use args::*;

pub(crate) mod internal;
pub(crate) mod server;

use internal::Internal;
use macros_rs::{crashln, string, ternary};
use pmc::{helpers, process::Runner};
use std::env;

fn format(server_name: &String) -> (String, String) {
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

pub fn start(name: &Option<String>, args: &Args, watch: &Option<String>, server_name: &String) {
    let runner = Runner::new();
    let (kind, list_name) = format(server_name);

    match args {
        Args::Id(id) => Internal { id: *id, runner, server_name, kind }.restart(name, watch),
        Args::Script(script) => match runner.find(&script, server_name) {
            Some(id) => Internal { id, runner, server_name, kind }.restart(name, watch),
            None => Internal { id: 0, runner, server_name, kind }.create(script, name, watch),
        },
    }

    Internal::list(&string!("default"), &list_name);
}

pub fn stop(item: &Item, server_name: &String) {
    let runner: Runner = Runner::new();
    let (kind, list_name) = format(server_name);

    match item {
        Item::Id(id) => Internal { id: *id, runner, server_name, kind }.stop(),
        Item::Name(name) => match runner.find(&name, server_name) {
            Some(id) => Internal { id, runner, server_name, kind }.stop(),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
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
