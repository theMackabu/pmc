use colored::Colorize;
use inquire::{Confirm, Password, PasswordDisplayMode, Select, Text};
use macros_rs::{crashln, string};
use std::{collections::BTreeMap, fs::write};

use pmc::{
    config::{
        self,
        structs::{Server, Servers},
    },
    helpers,
};

fn save(servers: BTreeMap<String, Server>) {
    match home::home_dir() {
        Some(path) => {
            let path = path.display();
            let config_path = format!("{path}/.pmc/servers.toml");

            let contents = match toml::to_string(&Servers {
                servers: Some(servers),
            }) {
                Ok(contents) => contents,
                Err(err) => crashln!(
                    "{} Cannot parse servers.\n{}",
                    *helpers::FAIL,
                    string!(err).white()
                ),
            };

            if let Err(err) = write(&config_path, contents) {
                crashln!(
                    "{} Error writing servers.\n{}",
                    *helpers::FAIL,
                    string!(err).white()
                )
            }
        }
        None => crashln!("{} Impossible to get your home directory", *helpers::FAIL),
    }
}

#[derive(Debug)]
struct ServerOption {
    name: String,
    formatted: String,
}

impl std::fmt::Display for ServerOption {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.formatted, f)
    }
}

pub fn list(format: &String, log_level: Option<log::Level>) {
    let servers = config::servers().servers.take().unwrap_or_default();

    let options: Vec<_> = servers
        .iter()
        .map(|(key, server)| {
            let verbose = match log_level {
                Some(_) => format!("({})", server.address),
                None => string!(),
            };

            ServerOption {
                name: key.clone(),
                formatted: format!("{} {}", key.to_string().bright_yellow(), verbose.white()),
            }
        })
        .collect();

    match Select::new("Select a server:", options).prompt() {
        Ok(server) => super::internal::Internal::list(format, &server.name),
        Err(_) => crashln!("{}", "Canceled...".white()),
    }
}

pub fn new() {
    let (name, address, token);
    let mut servers = config::servers().servers.take().unwrap_or_default();

    match Text::new("Server Name:").prompt() {
        Ok(ans) => name = ans,
        Err(_) => crashln!("{}", "Canceled...".white()),
    }

    match Text::new("Server Address:").prompt() {
        Ok(ans) => address = ans,
        Err(_) => crashln!("{}", "Canceled...".white()),
    }

    match Password::new("Server Token:")
        .with_display_toggle_enabled()
        .with_formatter(&|_| String::from("[hidden]"))
        .with_display_mode(PasswordDisplayMode::Masked)
        .without_confirmation()
        .prompt()
    {
        Ok(ans) => match ans.as_str() {
            "" => token = None,
            ans => token = Some(string!(ans)),
        },
        Err(_) => crashln!("{}", "Canceled...".white()),
    }

    match Confirm::new("Add server? (y/n)").prompt() {
        Err(_) => crashln!("{}", "Canceled...".white()),
        Ok(false) => {}
        Ok(true) => {
            if name.is_empty() || address.is_empty() {
                crashln!("{} Failed to add new server", *helpers::FAIL)
            } else {
                servers.insert(name, Server { address, token });
                save(servers);
                println!("{} Added new server", *helpers::SUCCESS)
            }
        }
    }
}

pub fn remove(name: &String) {
    let mut servers = config::servers().servers.take().unwrap_or_default();

    if servers.contains_key(name) {
        match Confirm::new(&format!("Remove server {name}? (y/n)")).prompt() {
            Err(_) => crashln!("{}", "Canceled...".white()),
            Ok(false) => {}
            Ok(true) => {
                servers.remove(name);
                save(servers);
                println!("{} Removed server (name={name})", *helpers::SUCCESS);
            }
        }
    } else {
        println!("{} Server {name} does not exist", *helpers::FAIL);
    }
}

pub fn default(name: &Option<String>) {
    let servers = config::servers().servers.take().unwrap_or_default();

    let name = match name {
        Some(name) => name.as_str(),
        None => "local",
    };

    if servers.contains_key(name) || name == "internal" || name == "local" {
        config::read().set_default(string!(name)).save();
        println!("{} Set default server to {name}", *helpers::SUCCESS)
    } else {
        println!("{} Server {name} does not exist", *helpers::FAIL);
    }
}
