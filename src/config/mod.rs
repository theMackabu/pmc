pub mod structs;

use crate::file::{self, Exists};
use crate::helpers;

use colored::Colorize;
use macros_rs::{crashln, string};
use std::fs;
use structs::{Config, Daemon, Runner};

pub fn read() -> Config {
    match home::home_dir() {
        Some(path) => {
            let path = path.display();
            let config_path = format!("{path}/.pmc/config.toml");

            if !Exists::file(config_path.clone()).unwrap() {
                let config = Config {
                    runner: Runner {
                        shell: string!("/bin/bash"),
                        args: vec![string!("bash"), string!("-c")],
                        log_path: format!("{path}/.pmc/logs"),
                    },
                    daemon: Daemon {
                        interval: 1000,
                        kind: string!("default"),
                    },
                };

                let contents = match toml::to_string(&config) {
                    Ok(contents) => contents,
                    Err(err) => crashln!("{} Cannot parse config.\n{}", *helpers::FAIL, string!(err).white()),
                };

                if let Err(err) = fs::write(&config_path, contents) {
                    crashln!("{} Error writing config.\n{}", *helpers::FAIL, string!(err).white())
                }
                log::info!("created config file");
            }

            file::read(config_path)
        }
        None => crashln!("{} Impossible to get your home directory", *helpers::FAIL),
    }
}
