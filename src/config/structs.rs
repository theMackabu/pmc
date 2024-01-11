use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub runner: Runner,
    pub daemon: Daemon,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Runner {
    pub shell: String,
    pub args: Vec<String>,
    pub node: String,
    pub log_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Daemon {
    pub restarts: u64,
    pub interval: u64,
    pub kind: String,
    pub web: Web,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Web {
    pub ui: bool,
    pub api: bool,
    pub address: String,
    pub port: u64,
    pub secure: Secure,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Secure {
    pub enabled: bool,
    pub token: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Servers {
    pub servers: Option<BTreeMap<String, Server>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Server {
    pub address: String,
    pub token: Option<String>,
}

impl Server {
    pub fn get(&self) -> Self {
        Self {
            token: self.token.clone(),
            address: self.address.trim_end_matches('/').to_string(),
        }
    }
}
