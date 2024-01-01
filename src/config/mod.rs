pub mod structs;

use crate::file::{self, Exists};
use crate::helpers;

use colored::Colorize;
use macros_rs::{crashln, string};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use structs::{Config, Daemon, Runner, Secure, Web};

pub fn read() -> Config {
    match home::home_dir() {
        Some(path) => {
            let path = path.display();
            let config_path = format!("{path}/.pmc/config.toml");

            if !Exists::file(config_path.clone()).unwrap() {
                let config = Config {
                    runner: Runner {
                        shell: string!("bash"),
                        args: vec![string!("-c")],
                        node: string!("node"),
                        log_path: format!("{path}/.pmc/logs"),
                    },
                    daemon: Daemon {
                        restarts: 10,
                        interval: 1000,
                        kind: string!("default"),
                        web: Web {
                            ui: false,
                            api: false,
                            address: string!("0.0.0.0"),
                            path: None,
                            port: 5630,
                            secure: Secure { enabled: false, token: string!("") },
                        },
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

impl Config {
    pub fn get_address(&self) -> SocketAddr {
        let config_split: Vec<u8> = match self.daemon.web.address.as_str() {
            "localhost" => vec![127, 0, 0, 1],
            _ => self.daemon.web.address.split('.').map(|part| part.parse().expect("Failed to parse address part")).collect(),
        };

        let ipv4_address: Ipv4Addr = Ipv4Addr::from([config_split[0], config_split[1], config_split[2], config_split[3]]);
        let ip_address: IpAddr = IpAddr::from(ipv4_address);
        let port = self.daemon.web.port as u16;

        (ip_address, port).into()
    }

    pub fn get_path(&self) -> String { self.daemon.web.path.clone().unwrap_or(string!("/")) }
}
