use crate::helpers;
use global_placeholders::init;
use macros_rs::crashln;

pub fn init() {
    match home::home_dir() {
        Some(path) => {
            let base = format!("{}/.pmc/", path.display());
            let logs = format!("{}/.pmc/logs/", path.display());
            let dump = format!("{}/.pmc/dump.toml", path.display());

            init!("pmc.base", base);
            init!("pmc.logs", logs);
            init!("pmc.dump", dump);

            let out = format!("{logs}{{}}-out.log");
            let error = format!("{logs}{{}}-error.log");

            init!("pmc.logs.out", out);
            init!("pmc.logs.error", error);
        }
        None => crashln!("{} Impossible to get your home directory", *helpers::FAIL),
    }
}
