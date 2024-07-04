use colored::Colorize;
use macros_rs::{crashln, string};
use serde::Deserialize;
use std::{collections::HashMap, fs};

use pmc::{
    helpers,
    process::{Env, Runner},
};

#[derive(Deserialize, Debug)]
struct ProcessWrapper {
    #[serde(alias = "process")]
    list: HashMap<String, Process>,
}

#[derive(Deserialize, Debug)]
struct Process {
    script: String,
    server: Option<String>,
    watch: Option<Watch>,
    #[serde(default)]
    env: Env,
}

#[derive(Deserialize, Debug)]
struct Watch {
    path: String,
}

impl Process {
    fn get_watch_path(&self) -> Option<String> { self.watch.as_ref().and_then(|w| Some(w.path.clone())) }
}

pub fn read_hcl(path: &String) {
    let mut servers: Vec<String> = vec![];

    println!("{} Applying action importProcess", *helpers::SUCCESS);

    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => crashln!("{} Cannot read file to import.\n{}", *helpers::FAIL, string!(err).white()),
    };

    let hcl_parsed: ProcessWrapper = match hcl::from_str(&contents) {
        Ok(hcl) => hcl,
        Err(err) => crashln!("{} Cannot parse imported file.\n{}", *helpers::FAIL, string!(err).white()),
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
        .create(&item.script, &Some(name.clone()), &item.get_watch_path(), true);

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

    servers.iter().for_each(|server| super::Internal::list(&string!("default"), &server));
    println!("{} Applied startProcess to imported items", *helpers::SUCCESS);
}
