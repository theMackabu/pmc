pub mod structs;

use crate::{
    file::{self, Exists},
    helpers,
    process::RemoteConfig,
};

use colored::Colorize;
use macros_rs::{crashln, fmtstr, string};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use structs::prelude::*;

use std::{
    fs::write,
    net::{IpAddr, Ipv4Addr},
    path::Path,
};

pub fn from(address: &str, token: Option<&str>) -> Result<RemoteConfig, anyhow::Error> {
    let client = Client::new();
    let mut headers = HeaderMap::new();

    if let Some(token) = token {
        headers.insert(
            "token",
            HeaderValue::from_static(Box::leak(Box::from(token))),
        );
    }

    let response = client
        .get(fmtstr!("{address}/daemon/config"))
        .headers(headers)
        .send()?;
    let json = response.json::<RemoteConfig>()?;

    Ok(json)
}

pub fn read() -> Config {
    match home::home_dir() {
        Some(path) => {
            let path = path.display();

            let config_path = format!("{path}/.pmc/config.toml");

            if !Exists::check(&config_path).file() {
                let config = Config {
                    default: string!("local"),
                    runner: Runner {
                        shell: string!("/bin/sh"),
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
                            secure: Some(Secure {
                                enabled: false,
                                token: string!(""),
                            }),
                        },
                    },
                };

                let contents = match toml::to_string(&config) {
                    Ok(contents) => contents,
                    Err(err) => crashln!(
                        "{} Cannot parse config.\n{}",
                        *helpers::FAIL,
                        string!(err).white()
                    ),
                };

                if let Err(err) = write(&config_path, contents) {
                    crashln!(
                        "{} Error writing config.\n{}",
                        *helpers::FAIL,
                        string!(err).white()
                    )
                }
                log::info!("created config file");
            }

            file::read(config_path)
        }
        None => crashln!("{} Impossible to get your home directory", *helpers::FAIL),
    }
}

pub fn servers() -> Servers {
    match home::home_dir() {
        Some(path) => {
            let path = path.display();
            let config_path = format!("{path}/.pmc/servers.toml");

            if !Exists::check(&config_path).file() {
                if let Err(err) = write(&config_path, "") {
                    crashln!(
                        "{} Error writing servers.\n{}",
                        *helpers::FAIL,
                        string!(err).white()
                    )
                }
            }

            file::read(config_path)
        }
        None => crashln!("{} Impossible to get your home directory", *helpers::FAIL),
    }
}

impl Config {
    pub fn check_shell_absolute(&self) -> bool {
        Path::new(&self.runner.shell).is_absolute()
    }

    pub fn get_address(&self) -> rocket::figment::Figment {
        let config_split: Vec<u8> = match self.daemon.web.address.as_str() {
            "localhost" => vec![127, 0, 0, 1],
            _ => self
                .daemon
                .web
                .address
                .split('.')
                .map(|part| part.parse().expect("Failed to parse address part"))
                .collect(),
        };

        let ipv4_address: Ipv4Addr = Ipv4Addr::from([
            config_split[0],
            config_split[1],
            config_split[2],
            config_split[3],
        ]);
        let ip_address: IpAddr = IpAddr::from(ipv4_address);

        rocket::Config::figment()
            .merge(("port", self.daemon.web.port))
            .merge(("address", ip_address))
    }

    pub fn save(&self) {
        match home::home_dir() {
            Some(path) => {
                let path = path.display();
                let config_path = format!("{path}/.pmc/config.toml");

                let contents = match toml::to_string(&self) {
                    Ok(contents) => contents,
                    Err(err) => crashln!(
                        "{} Cannot parse config.\n{}",
                        *helpers::FAIL,
                        string!(err).white()
                    ),
                };

                if let Err(err) = write(&config_path, contents) {
                    crashln!(
                        "{} Error writing config.\n{}",
                        *helpers::FAIL,
                        string!(err).white()
                    )
                }
            }
            None => crashln!("{} Impossible to get your home directory", *helpers::FAIL),
        }
    }

    pub fn set_default(mut self, name: String) -> Self {
        self.default = string!(name);
        self
    }

    pub fn fmt_address(&self) -> String {
        format!(
            "{}:{}",
            self.daemon.web.address.clone(),
            self.daemon.web.port
        )
    }

    pub fn get_path(&self) -> String {
        self.daemon.web.path.clone().unwrap_or(string!("/"))
    }
}
