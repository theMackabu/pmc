use global_placeholders::init;
use macros_rs::crashln;

pub fn init() {
    match home::home_dir() {
        Some(path) => {
            let logs = format!("{}/.pmc/logs/{{}}", path.display());
            let dump = format!("{}/.pmc/dump.toml", path.display());

            init!("pmc.logs", logs);
            init!("pmc.dump", dump);
        }
        None => crashln!("Impossible to get your home dir."),
    }
}
