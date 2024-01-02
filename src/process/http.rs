use crate::process::{Process, Remote};
use macros_rs::{fmtstr, string};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct ActionBody {
    pub method: String,
}

fn client(token: &Option<String>) -> (Client, HeaderMap) {
    let client = Client::new();
    let mut headers = HeaderMap::new();

    if let Some(token) = token {
        headers.insert(AUTHORIZATION, HeaderValue::from_static(fmtstr!("token {token}")));
    }

    return (client, headers);
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
