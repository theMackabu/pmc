use crate::{
    file::{self, Exists},
    helpers, log,
    process::{id::Id, Runner},
};

use colored::Colorize;
use global_placeholders::global;
use macros_rs::{crashln, string};
use std::{collections::BTreeMap, fs};

pub fn read() -> Runner {
    if !Exists::file(global!("pmc.dump")).unwrap() {
        let runner = Runner {
            id: Id::new(0),
            list: BTreeMap::new(),
        };

        write(&runner);
        log!("created dump file");
    }

    file::read_rmp(global!("pmc.dump"))
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
