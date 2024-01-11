mod cli;
mod daemon;
mod globals;
mod webui;

use crate::cli::Args;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{LogLevel, Verbosity};
use macros_rs::{str, string, then};

fn validate_id_script(s: &str) -> Result<Args, String> {
    if let Ok(id) = s.parse::<usize>() {
        Ok(Args::Id(id))
    } else {
        Ok(Args::Script(s.to_owned()))
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct NoneLevel;
impl LogLevel for NoneLevel {
    fn default() -> Option<log::Level> { None }
}

#[derive(Parser)]
#[command(version = str!(cli::get_version(false)))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[clap(flatten)]
    verbose: Verbosity<NoneLevel>,
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
        /// Server
        #[arg(short, long, default_value_t = string!("internal"))]
        server: String,
    },

    /// Stop/Kill a process
    #[command(alias = "kill")]
    Stop {
        id: usize,
        /// Server
        #[arg(short, long, default_value_t = string!("internal"))]
        server: String,
    },

    /// Stop then remove a process
    #[command(alias = "rm")]
    Remove {
        id: usize,
        /// Server
        #[arg(short, long, default_value_t = string!("internal"))]
        server: String,
    },

    /// Get env of a process
    #[command(alias = "cmdline")]
    Env {
        id: usize,
        /// Server
        #[arg(short, long, default_value_t = string!("internal"))]
        server: String,
    },

    /// Get information of a process
    #[command(alias = "info")]
    Details {
        id: usize,
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
        format: String,
        /// Server
        #[arg(short, long, default_value_t = string!("internal"))]
        server: String,
    },

    /// List all processes
    #[command(alias = "ls")]
    List {
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
        format: String,
        /// Server
        #[arg(short, long, default_value_t = string!("all"))]
        server: String,
    },

    /// Get logs from a process
    Logs {
        id: usize,
        #[arg(long, default_value_t = 15, help = "")]
        lines: usize,
        /// Server
        #[arg(short, long, default_value_t = string!("internal"))]
        server: String,
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
        Commands::Start { name, args, watch, server } => cli::start(name, args, watch, server),
        Commands::Stop { id, server } => cli::stop(id, server),
        Commands::Remove { id, server } => cli::remove(id, server),
        Commands::Env { id, server } => cli::env(id, server),
        Commands::Details { id, format, server } => cli::info(id, format, server),
        Commands::List { format, server } => cli::list(format, server),
        Commands::Logs { id, lines, server } => cli::logs(id, lines, server),

        Commands::Daemon { command } => match command {
            Daemon::Stop => daemon::stop(),
            Daemon::Reset => daemon::reset(),
            Daemon::Health { format } => daemon::health(format),
            Daemon::Restore { api, webui } => daemon::restart(api, webui, level.as_str() != "OFF"),
        },
    };

    if !matches!(&cli.command, Commands::Daemon { .. }) {
        then!(!daemon::pid::exists(), daemon::restart(&false, &false, false));
    }
}
