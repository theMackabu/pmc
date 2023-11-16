mod cli;
mod globals;
mod helpers;
mod process;
mod structs;

use crate::helpers::Exists;
use crate::structs::Args;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use global_placeholders::global;
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
enum Daemon {
    StartAll,
    StopAll,
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
    #[command(alias = "kill")]
    Stop { id: usize },
    #[command(alias = "rm")]
    Remove { id: usize },
    #[command(alias = "info")]
    // pmc restore command
    Details { id: usize },
    #[command(alias = "ls")]
    List {
        #[arg(long, default_value_t = string!(""), help = "format output")]
        format: String,
    },
    Daemon {
        #[command(subcommand)]
        command: Daemon,
    },
}

fn main() {
    // make sure process is running, if not, restart
    // make this daemon based.

    let cli = Cli::parse();

    globals::init();
    env_logger::Builder::new().filter_level(cli.verbose.log_level_filter()).init();

    if !Exists::folder(global!("pmc.logs")).unwrap() {
        std::fs::create_dir_all(global!("pmc.logs")).unwrap();
        log::info!("created PMC log directory");
    }

    match &cli.command {
        Commands::Start { name, args } => cli::start(name, args),
        Commands::Stop { id } => cli::stop(id),
        Commands::Remove { id } => cli::remove(id),
        Commands::Details { id } => cli::info(id),
        Commands::List { format } => cli::list(format),
        Commands::Daemon { command } => match command {
            Daemon::StartAll => {}
            Daemon::StopAll => {}
        },
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
