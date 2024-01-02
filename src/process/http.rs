use crate::process::{Process, Remote};
use macros_rs::fmtstr;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use std::collections::BTreeMap;

pub fn list(remote: &Remote) -> Result<BTreeMap<usize, Process>, anyhow::Error> {
    let client = Client::new();
    let mut headers = HeaderMap::new();
    let Remote { address, token } = remote;

    if let Some(token) = token {
        headers.insert(AUTHORIZATION, HeaderValue::from_static(fmtstr!("token {token}")));
    }

    let response = client.get(fmtstr!("{address}/list")).headers(headers).send()?;
    Ok(response.json()?)
}
