use crate::helpers::Exists;
use crate::helpers::Id;
use crate::process::Runner;

use colored::Colorize;
use global_placeholders::global;
use macros_rs::{crashln, string};
use std::{collections::BTreeMap, fs};

pub fn read() -> Runner {
    if !Exists::folder(global!("pmc.base")).unwrap() {
        fs::create_dir_all(global!("pmc.base")).unwrap();
        log::info!("created pmc base dir");
    }

    if !Exists::file(global!("pmc.dump")).unwrap() {
        let runner = Runner {
            log_path: string!(""),
            id: Id::new(0),
            process_list: BTreeMap::new(),
        };

        write(&runner);
        log::info!("created dump file");
    }

    let contents = match fs::read_to_string(global!("pmc.dump")) {
        Ok(contents) => contents,
        Err(err) => crashln!("Cannot find dumpfile.\n{}", string!(err).white()),
    };

    match toml::from_str(&contents).map_err(|err| string!(err)) {
        Ok(parsed) => parsed,
        Err(err) => crashln!("Cannot read dumpfile.\n{}", err.white()),
    }
}

pub fn write(dump: &Runner) {
    if !Exists::folder(global!("pmc.base")).unwrap() {
        fs::create_dir_all(global!("pmc.base")).unwrap();
        log::info!("created pmc base dir");
    }

    let contents = match toml::to_string(dump) {
        Ok(contents) => contents,
        Err(err) => crashln!("Cannot parse dump.\n{}", string!(err).white()),
    };

    if let Err(err) = fs::write(global!("pmc.dump"), contents) {
        crashln!("Error writing dumpfile.\n{}", string!(err).white())
    }
}
