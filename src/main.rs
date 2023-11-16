mod cli;
mod file;
mod globals;
mod helpers;
mod process;
mod structs;

use crate::file::Exists;
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
    /// Start all processes
    StartAll,
    /// Stop all processes
    StopAll,
    /// Reset process index
    ResetIndex,
    /// Check daemon
    Health,
}

// add pmc restore command
#[derive(Subcommand)]
enum Commands {
    /// Start/Restart a process
    #[command(alias = "restart")]
    Start {
        #[arg(long, help = "process name")]
        name: Option<String>,
        #[clap(value_parser = validate_id_script)]
        args: Option<Args>,
    },

    /// Stop/Kill a process
    #[command(alias = "kill")]
    Stop { id: usize },

    /// Stop then remove a process
    #[command(alias = "rm")]
    Remove { id: usize },

    /// Get env of a process
    #[command(alias = "cmdline")]
    Env { id: usize },

    /// Get information of a process
    #[command(alias = "info")]
    Details { id: usize },

    /// List all processes
    #[command(alias = "ls")]
    List {
        #[arg(long, default_value_t = string!(""), help = "format output")]
        format: String,
    },

    /// Get logs from a process
    Logs {
        id: usize,
        #[arg(long, default_value_t = 15, help = "")]
        lines: usize,
    },

    /// Daemon management
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
        Commands::Env { id } => cli::env(id),
        Commands::Details { id } => cli::info(id),
        Commands::List { format } => cli::list(format),
        Commands::Logs { id, lines } => cli::logs(id, lines),
        Commands::Daemon { command } => match command {
            Daemon::StartAll => {}
            Daemon::StopAll => {}
            Daemon::ResetIndex => {}
            Daemon::Health => {}
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
