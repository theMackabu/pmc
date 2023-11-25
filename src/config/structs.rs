use serde::{Deserialize, Serialize};

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
    pub api: Api,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Api {
    pub enabled: bool,
    pub address: String,
    pub port: u64,
    pub secure: Secure,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Secure {
    pub enabled: bool,
    pub token: String,
}
