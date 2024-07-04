use crate::process::Remote;
use macros_rs::{fmtstr, string};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct ActionBody {
    pub method: String,
}

pub struct LogResponse {
    pub path: &'static str,
    pub lines: Vec<String>,
}

#[derive(Serialize)]
struct CreateBody<'c> {
    pub name: &'c String,
    pub script: &'c String,
    pub path: PathBuf,
    pub watch: &'c Option<String>,
}

pub mod sync {
    use reqwest::blocking::Client;
    use reqwest::header::{HeaderMap, HeaderValue};

    pub use reqwest::blocking::Response;
    pub fn client(token: &Option<String>) -> (Client, HeaderMap) {
        let client = Client::new();
        let mut headers = HeaderMap::new();

        if let Some(token) = token {
            headers.insert("token", HeaderValue::from_str(&token).unwrap());
        }

        return (client, headers);
    }
}

pub async fn client(token: &Option<String>) -> (Client, HeaderMap) {
    let client = Client::new();
    let mut headers = HeaderMap::new();

    if let Some(token) = token {
        headers.insert("token", HeaderValue::from_str(&token).unwrap());
    }

    return (client, headers);
}

pub fn info(Remote { address, token, .. }: &Remote, id: usize) -> Result<sync::Response, anyhow::Error> {
    let (client, headers) = sync::client(token);
    Ok(client.get(fmtstr!("{address}/process/{id}/info")).headers(headers).send()?)
}

pub fn logs(Remote { address, token, .. }: &Remote, id: usize, kind: &str) -> Result<LogResponse, anyhow::Error> {
    let (client, headers) = sync::client(token);
    let response = client.get(fmtstr!("{address}/process/{id}/logs/{kind}/raw")).headers(headers).send()?;
    let log = response.text()?;

    Ok(LogResponse {
        lines: log.lines().skip(1).map(|line| line.to_string()).collect::<Vec<String>>(),
        path: Box::leak(Box::from(log.lines().next().unwrap_or("").split_whitespace().last().unwrap_or(""))),
    })
}

pub fn create(Remote { address, token, .. }: &Remote, name: &String, script: &String, path: PathBuf, watch: &Option<String>) -> Result<sync::Response, anyhow::Error> {
    let (client, headers) = sync::client(token);
    let content = CreateBody { name, script, path, watch };

    Ok(client.post(fmtstr!("{address}/process/create")).json(&content).headers(headers).send()?)
}

pub fn restart(Remote { address, token, .. }: &Remote, id: usize) -> Result<sync::Response, anyhow::Error> {
    let (client, headers) = sync::client(token);
    let content = ActionBody { method: string!("restart") };

    Ok(client.post(fmtstr!("{address}/process/{id}/action")).json(&content).headers(headers).send()?)
}

pub fn rename(Remote { address, token, .. }: &Remote, id: usize, name: String) -> Result<sync::Response, anyhow::Error> {
    let (client, headers) = sync::client(token);
    Ok(client.post(fmtstr!("{address}/process/{id}/rename")).body(name).headers(headers).send()?)
}

// merge into one function
pub fn stop(Remote { address, token, .. }: &Remote, id: usize) -> Result<sync::Response, anyhow::Error> {
    let (client, headers) = sync::client(token);
    let content = ActionBody { method: string!("stop") };

    Ok(client.post(fmtstr!("{address}/process/{id}/action")).json(&content).headers(headers).send()?)
}

pub fn remove(Remote { address, token, .. }: &Remote, id: usize) -> Result<sync::Response, anyhow::Error> {
    let (client, headers) = sync::client(token);
    let content = ActionBody { method: string!("remove") };

    Ok(client.post(fmtstr!("{address}/process/{id}/action")).json(&content).headers(headers).send()?)
}

pub fn flush(Remote { address, token, .. }: &Remote, id: usize) -> Result<sync::Response, anyhow::Error> {
    let (client, headers) = sync::client(token);
    let content = ActionBody { method: string!("flush") };

    Ok(client.post(fmtstr!("{address}/process/{id}/action")).json(&content).headers(headers).send()?)
}

pub fn clear_env(Remote { address, token, .. }: &Remote, id: usize) -> Result<sync::Response, anyhow::Error> {
    let (client, headers) = sync::client(token);
    let content = ActionBody { method: string!("clear_env") };

    Ok(client.post(fmtstr!("{address}/process/{id}/action")).json(&content).headers(headers).send()?)
}
