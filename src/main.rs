mod cli;
mod daemon;
mod globals;

use crate::cli::Args;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
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
    Restore {
        /// Daemon api
        #[arg(long)]
        api: bool,
        /// WebUI using api
        #[arg(long)]
        webui: bool,
    },
    /// Check daemon
    #[command(alias = "info", alias = "status")]
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
        /// Process name
        #[arg(long)]
        name: Option<String>,
        #[clap(value_parser = validate_id_script)]
        args: Option<Args>,
        /// Watch to reload path
        #[arg(long)]
        watch: Option<String>,
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
    let cli = Cli::parse();
    let mut env = env_logger::Builder::new();
    let level = cli.verbose.log_level_filter();

    globals::init();
    env.filter_level(level).init();

    match &cli.command {
        Commands::Start { name, args, watch } => cli::start(name, args, watch),
        Commands::Stop { id } => cli::stop(id),
        Commands::Remove { id } => cli::remove(id),
        Commands::Env { id } => cli::env(id),
        Commands::Details { id, format } => cli::info(id, format),
        Commands::List { format } => cli::list(format),
        Commands::Logs { id, lines } => cli::logs(id, lines),

        Commands::Daemon { command } => match command {
            Daemon::Stop => daemon::stop(),
            Daemon::Reset => daemon::reset(),
            Daemon::Health { format } => daemon::health(format),
            Daemon::Restore { api, webui } => daemon::restart(api, webui, level.as_str() != "ERROR"),
        },
    };

    if !matches!(&cli.command, Commands::Daemon { .. }) {
        then!(!daemon::pid::exists(), daemon::start(false));
    }
}
