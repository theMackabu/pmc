use crate::file::{self, Exists};
use crate::helpers::{self, Id};
use crate::process::Runner;

use colored::Colorize;
use global_placeholders::global;
use macros_rs::{crashln, string};
use std::{collections::BTreeMap, fs};

pub fn read() -> Runner {
    if !Exists::file(global!("pmc.dump")).unwrap() {
        let runner = Runner {
            id: Id::new(0),
            process_list: BTreeMap::new(),
        };

        write(&runner);
        log::info!("created dump file");
    }

    file::read(global!("pmc.dump"))
}

pub fn write(dump: &Runner) {
    let contents = match toml::to_string(dump) {
        Ok(contents) => contents,
        Err(err) => crashln!("{} Cannot parse dump.\n{}", *helpers::FAIL, string!(err).white()),
    };

    if let Err(err) = fs::write(global!("pmc.dump"), contents) {
        crashln!("{} Error writing dumpfile.\n{}", *helpers::FAIL, string!(err).white())
    }
}
