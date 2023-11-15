mod cli;
mod globals;
mod helpers;
mod process;
mod structs;

use crate::structs::Args;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use macros_rs::{str, string};

fn validate_id_script(s: &str) -> Result<Args, String> {
    if let Ok(id) = s.parse::<usize>() {
        Ok(Args::Id(id))
    } else {
        Ok(Args::Script(s.to_owned()))
    }
}

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
    #[command(alias = "restart")]
    Start {
        #[arg(long, help = "process name")]
        name: Option<String>,
        #[clap(value_parser = validate_id_script)]
        args: Option<Args>,
    },
    Stop {
        id: usize,
    },
    #[command(alias = "ls")]
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
        Commands::Start { name, args } => cli::start(name, args),
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
