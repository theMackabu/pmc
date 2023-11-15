mod cli;
mod globals;
mod helpers;
mod process;

use crate::process::Runner;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use macros_rs::{str, string, ternary};
use std::{thread, time::Duration};

#[derive(Parser)]
#[command(version = str!(cli::get_version(false)))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[clap(flatten)]
    verbose: Verbosity,
}

#[derive(Subcommand)]
enum Commands {
    Start {
        id: usize,
    },
    Stop {
        id: usize,
    },
    List {
        #[arg(long, default_value_t = string!("default"), help = "format output")]
        format: String,
    },
}

fn main() {
    let cli = Cli::parse();

    globals::init();
    env_logger::Builder::new().filter_level(cli.verbose.log_level_filter()).init();

    match &cli.command {
        Commands::Start { id } => println!("{id}"),
        Commands::Stop { id } => println!("{id}"),
        Commands::List { format } => println!("{format}"),
    }

    // save in ~/.pmc/dump.toml
    // logs in ~/.pmc/logs
    // use global placeholders for home crate
    // use psutil for memory and cpu usage (in PAW)
    // create log dir if not exist
    // use clap cli and rataui for ui
    //    (pmc ls, pmc list, pmc ls --format=json, pmc list --format=json)
    //    [use clap command alias]

    let mut runner = Runner::new("tests/logs");

    runner.start("example", "node tests/index.js");
    println!("{:?}", runner.info(0));

    thread::sleep(Duration::from_millis(1000));

    runner.stop(0);
    println!("{:?}", runner.info(0));

    // runner.list().iter().for_each(|(id, item)| println!("id: {}\nname: {}", id, item.name));

    for (id, item) in runner.list() {
        println!("id: {id}\nname: {}\npid: {}\nstatus: {}", item.name, item.pid, ternary!(item.running, "online", "offline"));
    }
}

#[cxx::bridge]
pub mod service {
    unsafe extern "C++" {
        include!("pmc/src/include/process.h");
        include!("pmc/src/include/bridge.h");

        pub fn stop(pid: i64) -> i64;
        pub fn run(name: &str, log_path: &str, command: &str) -> i64;
    }
}
