use crate::{
    file::{self, Exists},
    helpers, log,
    process::{id::Id, Runner},
};

use colored::Colorize;
use global_placeholders::global;
use macros_rs::{crashln, fmtstr, string};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use std::{collections::BTreeMap, fs};

pub fn from<'r>(address: &str, token: Option<&str>) -> Result<Runner, anyhow::Error> {
    let client = Client::new();
    let mut headers = HeaderMap::new();

    if let Some(token) = token {
        headers.insert(AUTHORIZATION, HeaderValue::from_static(fmtstr!("token {token}")));
    }

    let response = client.get(fmtstr!("{address}/dump")).headers(headers).send()?;
    let bytes = response.bytes()?;

    Ok(file::from_rmp(&bytes))
}

pub fn read() -> Runner {
    if !Exists::file(global!("pmc.dump")).unwrap() {
        let runner = Runner {
            id: Id::new(0),
            list: BTreeMap::new(),
            remote: None,
        };

        write(&runner);
        log!("created dump file");
    }

    file::read_rmp(global!("pmc.dump"))
}

pub fn raw() -> Vec<u8> {
    if !Exists::file(global!("pmc.dump")).unwrap() {
        let runner = Runner {
            id: Id::new(0),
            list: BTreeMap::new(),
            remote: None,
        };

        write(&runner);
        log!("created dump file");
    }

    file::raw(global!("pmc.dump"))
}

pub fn write(dump: &Runner) {
    let encoded: Vec<u8> = match rmp_serde::to_vec(&dump) {
        Ok(contents) => contents,
        Err(err) => crashln!("{} Cannot encode dump.\n{}", *helpers::FAIL, string!(err).white()),
    };

    if let Err(err) = fs::write(global!("pmc.dump"), encoded) {
        crashln!("{} Error writing dumpfile.\n{}", *helpers::FAIL, string!(err).white())
    }
}
