use global_placeholders::init;
use macros_rs::crashln;
use pmc::{config, file::Exists, helpers};
use std::fs;

pub fn init() {
    match home::home_dir() {
        Some(path) => {
            let path = path.display();
            if !Exists::check(&format!("{path}/.pmc/")).folder() {
                fs::create_dir_all(format!("{path}/.pmc/")).unwrap();
                log::info!("created pmc base dir");
            }

            let config = config::read();
            if !Exists::check(&config.runner.log_path).folder() {
                fs::create_dir_all(&config.runner.log_path).unwrap();
                log::info!("created pmc log dir");
            }

            init!("pmc.base", format!("{path}/.pmc/"));
            init!("pmc.log", format!("{path}/.pmc/pmc.log"));
            init!("pmc.pid", format!("{path}/.pmc/daemon.pid"));
            init!("pmc.dump", format!("{path}/.pmc/process.dump"));

            init!("pmc.daemon.kind", config.daemon.kind);
            init!("pmc.daemon.log", format!("{path}/.pmc/daemon.log"));

            let out = format!("{}/{{}}-out.log", config.runner.log_path);
            let error = format!("{}/{{}}-error.log", config.runner.log_path);

            init!("pmc.logs.out", out);
            init!("pmc.logs.error", error);
        }
        None => crashln!("{} Impossible to get your home directory", *helpers::FAIL),
    }
}
