use super::Item;
use colored::Colorize;
use macros_rs::{crashln, string};
use serde::{Deserialize, Serialize};

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::prelude::*,
};

use pmc::{
    file::Exists,
    helpers,
    process::{Env, Runner},
};

#[derive(Deserialize, Debug)]
struct ProcessWrapper {
    #[serde(alias = "process")]
    list: HashMap<String, Process>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Process {
    script: String,
    server: Option<String>,
    watch: Option<Watch>,
    #[serde(default)]
    env: Env,
}

#[derive(Serialize, Deserialize, Debug)]
struct Watch {
    path: String,
}

impl Process {
    fn get_watch_path(&self) -> Option<String> {
        self.watch.as_ref().map(|w| w.path.clone())
    }
}

pub fn read_hcl(path: &String) {
    let mut servers: Vec<String> = vec![];

    println!("{} Applying action importProcess", *helpers::SUCCESS);

    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => crashln!(
            "{} Cannot read file to import.\n{}",
            *helpers::FAIL,
            string!(err).white()
        ),
    };

    let hcl_parsed: ProcessWrapper = match hcl::from_str(&contents) {
        Ok(hcl) => hcl,
        Err(err) => crashln!(
            "{} Cannot parse imported file.\n{}",
            *helpers::FAIL,
            string!(err).white()
        ),
    };

    for (name, item) in hcl_parsed.list {
        let mut runner = Runner::new();
        let server_name = &item.server.clone().unwrap_or("local".into());
        let (kind, list_name) = super::format(server_name);

        runner = super::Internal {
            id: 0,
            server_name,
            kind: kind.clone(),
            runner: runner.clone(),
        }
        .create(
            &item.script,
            &Some(name.clone()),
            &item.get_watch_path(),
            true,
        );

        println!("{} Imported {kind}process {name}", *helpers::SUCCESS);

        match runner.find(&name, server_name) {
            Some(id) => {
                let mut p = runner.get(id);
                p.stop();
                p.set_env(item.env);
                p.restart();
            }
            None => crashln!("{} Failed to write to ({name})", *helpers::FAIL),
        }

        if !servers.contains(&list_name) {
            servers.push(list_name);
        }
    }

    servers
        .iter()
        .for_each(|server| super::Internal::list(&string!("default"), server));
    println!(
        "{} Applied startProcess to imported items",
        *helpers::SUCCESS
    );
}

pub fn export_hcl(item: &Item, path: &Option<String>) {
    println!("{} Applying action exportProcess", *helpers::SUCCESS);

    let runner = Runner::new();

    let fetch_process = |id: usize| {
        let process = runner.try_info(id);
        let mut watch_parsed = None;
        let mut env_parsed = HashMap::new();

        let current_env: HashMap<String, String> = std::env::vars().collect();
        let path = path
            .clone()
            .unwrap_or(format!("{}.hcl", process.name.clone()));

        if process.watch.enabled {
            watch_parsed = Some(Watch {
                path: process.watch.path.clone(),
            })
        }

        for (key, value) in process.env.clone() {
            if let Some(current_value) = current_env.get(&key) {
                if current_value != &value {
                    env_parsed.insert(key, value);
                }
            } else {
                env_parsed.insert(key, value);
            }
        }

        let data = hcl::block! {
            process (process.name.clone()) {
                script = (process.script.clone())
                server = ("")
                watch = (watch_parsed)
                env = (env_parsed)
            }
        };

        let serialized = hcl::to_string(&data).unwrap();

        if Exists::check(&path).file() {
            let mut file = OpenOptions::new().append(true).open(path.clone()).unwrap();
            if let Err(err) = writeln!(file, "{}", serialized) {
                crashln!(
                    "{} Error writing to file.\n{}",
                    *helpers::FAIL,
                    string!(err).white()
                )
            }
        } else if let Err(err) = fs::write(path.clone(), serialized) {
            crashln!(
                "{} Error writing file.\n{}",
                *helpers::FAIL,
                string!(err).white()
            )
        }

        println!("{} Exported process {id} to {path}", *helpers::SUCCESS);
    };

    match item {
        Item::Id(id) => fetch_process(*id),
        Item::Name(name) => match runner.find(name, &string!("internal")) {
            Some(id) => fetch_process(id),
            None => crashln!("{} Process ({name}) not found", *helpers::FAIL),
        },
    }
}
