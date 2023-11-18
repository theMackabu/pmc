mod cli;
mod daemon;
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
use macros_rs::{str, string, then};

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
    /// Reset process index
    #[command(alias = "clean")]
    Reset,
    /// Stop daemon
    #[command(alias = "kill")]
    Stop,
    /// Restart daemon
    #[command(alias = "restart", alias = "start")]
    Restore,
    /// Check daemon
    #[command(alias = "info")]
    Health {
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
        format: String,
    },
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
    Details {
        id: usize,
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
        format: String,
    },

    /// List all processes
    #[command(alias = "ls")]
    List {
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
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
    globals::init();

    let cli = Cli::parse();
    let mut env = env_logger::Builder::new();
    env.filter_level(cli.verbose.log_level_filter()).init();

    if !Exists::folder(global!("pmc.logs")).unwrap() {
        std::fs::create_dir_all(global!("pmc.logs")).unwrap();
        log::info!("Created PMC log directory");
    }

    match &cli.command {
        // add --watch
        Commands::Start { name, args } => cli::start(name, args),
        Commands::Stop { id } => cli::stop(id),
        Commands::Remove { id } => cli::remove(id),
        Commands::Env { id } => cli::env(id),
        Commands::Details { id, format } => cli::info(id, format),
        Commands::List { format } => cli::list(format),
        Commands::Logs { id, lines } => cli::logs(id, lines),

        Commands::Daemon { command } => {
            match command {
                Daemon::Reset => {}
                Daemon::Stop => daemon::stop(),
                Daemon::Restore => daemon::restart(),
                Daemon::Health { format } => daemon::health(format),
            };

            if !matches!(command, Daemon::Stop | Daemon::Health { .. }) {
                then!(!daemon::pid::exists(), daemon::start());
            }
        }
    };
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
