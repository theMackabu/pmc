use crate::process::{Process, Remote};
use macros_rs::{fmtstr, string};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Serialize)]
struct ActionBody {
    pub method: String,
}

#[derive(Serialize)]
struct CreateBody<'c> {
    pub name: &'c String,
    pub script: &'c String,
    pub path: PathBuf,
    pub watch: &'c Option<String>,
}

fn client(token: &Option<String>) -> (Client, HeaderMap) {
    let client = Client::new();
    let mut headers = HeaderMap::new();

    if let Some(token) = token {
        headers.insert(AUTHORIZATION, HeaderValue::from_static(fmtstr!("token {token}")));
    }

    return (client, headers);
}

pub fn create(Remote { address, token }: &Remote, name: &String, script: &String, path: PathBuf, watch: &Option<String>) -> Result<Response, anyhow::Error> {
    let (client, headers) = client(token);
    let content = CreateBody { name, script, path, watch };

    Ok(client.post(fmtstr!("{address}/process/create")).json(&content).headers(headers).send()?)
}

pub fn restart(Remote { address, token }: &Remote, id: usize) -> Result<Response, anyhow::Error> {
    let (client, headers) = client(token);
    let content = ActionBody { method: string!("restart") };

    Ok(client.post(fmtstr!("{address}/process/{id}/action")).json(&content).headers(headers).send()?)
}

pub fn stop(Remote { address, token }: &Remote, id: usize) -> Result<Response, anyhow::Error> {
    let (client, headers) = client(token);
    let content = ActionBody { method: string!("stop") };

    Ok(client.post(fmtstr!("{address}/process/{id}/action")).json(&content).headers(headers).send()?)
}

pub fn remove(Remote { address, token }: &Remote, id: usize) -> Result<Response, anyhow::Error> {
    let (client, headers) = client(token);
    let content = ActionBody { method: string!("remove") };

    Ok(client.post(fmtstr!("{address}/process/{id}/action")).json(&content).headers(headers).send()?)
}
