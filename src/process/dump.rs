use crate::file::Exists;
use crate::helpers::{self, Id};
use crate::process::Runner;

use colored::Colorize;
use global_placeholders::global;
use macros_rs::{crashln, string};
use std::{collections::BTreeMap, fs, thread::sleep, time::Duration};

pub fn read() -> Runner {
    if !Exists::folder(global!("pmc.base")).unwrap() {
        fs::create_dir_all(global!("pmc.base")).unwrap();
        log::info!("created pmc base dir");
    }

    if !Exists::file(global!("pmc.dump")).unwrap() {
        let runner = Runner {
            id: Id::new(0),
            log_path: global!("pmc.logs"),
            process_list: BTreeMap::new(),
        };

        write(&runner);
        log::info!("created dump file");
    }

    let mut retry_count = 0;
    let max_retries = 5;

    let contents = loop {
        match fs::read_to_string(global!("pmc.dump")) {
            Ok(contents) => break contents,
            Err(err) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    crashln!("{} Cannot find dumpfile.\n{}", *helpers::FAIL, string!(err).white());
                } else {
                    println!("{} Error reading dumpfile. Retrying... (Attempt {}/{})", *helpers::FAIL, retry_count, max_retries);
                }
            }
        }
        sleep(Duration::from_secs(1));
    };

    retry_count = 0;

    loop {
        match toml::from_str(&contents).map_err(|err| string!(err)) {
            Ok(parsed) => break parsed,
            Err(err) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    crashln!("{} Cannot parse dumpfile.\n{}", *helpers::FAIL, err.white());
                } else {
                    println!("{} Error parsing dumpfile. Retrying... (Attempt {}/{})", *helpers::FAIL, retry_count, max_retries);
                }
            }
        }
        sleep(Duration::from_secs(1));
    }
}

pub fn write(dump: &Runner) {
    if !Exists::folder(global!("pmc.base")).unwrap() {
        fs::create_dir_all(global!("pmc.base")).unwrap();
        log::info!("created pmc base dir");
    }

    let contents = match toml::to_string(dump) {
        Ok(contents) => contents,
        Err(err) => crashln!("{} Cannot parse dump.\n{}", *helpers::FAIL, string!(err).white()),
    };

    if let Err(err) = fs::write(global!("pmc.dump"), contents) {
        crashln!("{} Error writing dumpfile.\n{}", *helpers::FAIL, string!(err).white())
    }
}
