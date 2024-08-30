use global_placeholders::init;
use macros_rs::{crashln, then};
use once_cell::sync::OnceCell;
use pmc::{config, file::Exists, helpers};
use serde::{Deserialize, Serialize};
use std::fs;
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Os {
    pub name: os_info::Type,
    pub version: String,
    pub arch: String,
    pub bitness: os_info::Bitness,
}

pub static OS_INFO: OnceCell<Os> = OnceCell::new();

pub fn get_os_info() -> &'static Os {
    OS_INFO.get_or_init(|| {
        let os = os_info::get();
        Os {
            name: os.os_type(),
            version: os.version().to_string(),
            arch: os.architecture().unwrap().into(),
            bitness: os.bitness(),
        }
    })
}

pub(crate) fn init() {
    match home::home_dir() {
        Some(path) => {
            let path = path.display();

            if !Exists::check(&format!("{path}/.pmc/")).folder() {
                fs::create_dir_all(format!("{path}/.pmc/")).unwrap();
                log::info!("created pmc base dir");
            }

            let config = config::read();
            then!(
                !config.check_shell_absolute(),
                println!(
                    "{} Shell is not an absolute path.\n {1} Please update this in {path}/.pmc/config.toml\n {1} Failure to update will prevent programs from restarting",
                    *helpers::WARN,
                    *helpers::WARN_STAR
                )
            );

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

pub(crate) fn defaults(name: &Option<String>) -> String {
    match name {
        Some(name) => name.clone(),
        None => config::read().default,
    }
}
