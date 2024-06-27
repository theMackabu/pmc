use macros_rs::{crashln, string};
use pmc::{config, file, helpers, log, process::Runner};
use regex::Regex;

pub struct Internal<'i> {
    pub id: usize,
    pub runner: Runner,
    pub kind: String,
    pub server_name: &'i String,
}

impl<'i> Internal<'i> {
    pub fn create(mut self, script: &String, name: &Option<String>, watch: &Option<String>) {
        let config = config::read();
        let name = match name {
            Some(name) => string!(name),
            None => string!(script.split_whitespace().next().unwrap_or_default()),
        };

        if matches!(&**self.server_name, "internal" | "local") {
            let pattern = Regex::new(r"(?m)^[a-zA-Z0-9]+(/[a-zA-Z0-9]+)*(\.js|\.ts)?$").unwrap();

            if pattern.is_match(script) {
                let script = format!("{} {script}", config.runner.node);
                self.runner.start(&name, &script, file::cwd(), watch).save();
            } else {
                self.runner.start(&name, script, file::cwd(), watch).save();
            }
        } else {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                match Runner::connect(self.server_name.clone(), server.get(), false) {
                    Some(mut remote) => remote.start(&name, script, file::cwd(), watch),
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name,)
            };
        }

        println!("{} Creating {}process with ({name})", *helpers::SUCCESS, self.kind);
        println!("{} {}created ({name}) ✓", *helpers::SUCCESS, self.kind);
    }

    pub fn restart(self, name: &Option<String>, watch: &Option<String>) {
        println!("{} Applying {}action restartProcess on ({})", *helpers::SUCCESS, self.kind, self.id);

        if matches!(&**self.server_name, "internal" | "local") {
            let mut item = self.runner.get(self.id);

            match watch {
                Some(path) => item.watch(path),
                None => item.disable_watch(),
            }

            name.as_ref().map(|n| item.rename(n.trim().replace("\n", "")));
            item.restart();

            log!("process started (id={})", self.id);
        } else {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                match Runner::connect(self.server_name.clone(), server.get(), false) {
                    Some(remote) => {
                        let mut item = remote.get(self.id);

                        name.as_ref().map(|n| item.rename(n.trim().replace("\n", "")));
                        item.restart();
                    }
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                }
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };
        }

        println!("{} restarted {}({}) ✓", *helpers::SUCCESS, self.kind, self.id);
    }

    pub fn stop(mut self) {
        println!("{} Applying {}action stopProcess on ({})", *helpers::SUCCESS, self.kind, self.id);

        if !matches!(&**self.server_name, "internal" | "local") {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                self.runner = match Runner::connect(self.server_name.clone(), server.get(), false) {
                    Some(remote) => remote,
                    None => crashln!("{} Failed to connect (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };
        }

        self.runner.get(self.id).stop();
        println!("{} stopped {}({}) ✓", *helpers::SUCCESS, self.kind, self.id);
        log!("process stopped {}(id={})", self.kind, self.id);
    }

    pub fn remove(mut self) {
        println!("{} Applying {}action removeProcess on ({})", *helpers::SUCCESS, self.kind, self.id);

        if !matches!(&**self.server_name, "internal" | "local") {
            let Some(servers) = config::servers().servers else {
                crashln!("{} Failed to read servers", *helpers::FAIL)
            };

            if let Some(server) = servers.get(self.server_name) {
                self.runner = match Runner::connect(self.server_name.clone(), server.get(), false) {
                    Some(remote) => remote,
                    None => crashln!("{} Failed to remove (name={}, address={})", *helpers::FAIL, self.server_name, server.address),
                };
            } else {
                crashln!("{} Server '{}' does not exist", *helpers::FAIL, self.server_name)
            };
        }

        self.runner.remove(self.id);
        println!("{} removed {}({}) ✓", *helpers::SUCCESS, self.kind, self.id);
        log!("process removed (id={})", self.id);
    }
}
