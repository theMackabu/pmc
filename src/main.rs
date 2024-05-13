mod cli;
mod daemon;
mod globals;
mod webui;

use crate::{cli::Args, globals::defaults};
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{LogLevel, Verbosity};
use macros_rs::{str, string, then};
use update_informer::{registry, Check};

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
    #[command(visible_alias = "clean")]
    Reset,
    /// Stop daemon
    #[command(visible_alias = "kill")]
    Stop,
    /// Restart daemon
    #[command(visible_alias = "restart", visible_alias = "start")]
    Restore {
        /// Daemon api
        #[arg(long)]
        api: bool,
        /// WebUI using api
        #[arg(long)]
        webui: bool,
    },
    /// Check daemon
    #[command(visible_alias = "info", visible_alias = "status")]
    Health {
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
        format: String,
    },
}

#[derive(Subcommand)]
enum Server {
    /// Add new server
    #[command(visible_alias = "add")]
    New,
    /// List servers
    #[command(visible_alias = "ls")]
    List {
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
        format: String,
    },
    /// Remove server
    #[command(visible_alias = "rm")]
    Remove {
        /// Server name
        name: String,
    },
    /// Set default server
    #[command(visible_alias = "set")]
    Default {
        /// Server name
        name: Option<String>,
    },
}

// add pmc restore command
#[derive(Subcommand)]
enum Commands {
    /// Start/Restart a process
    #[command(visible_alias = "restart")]
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
        #[arg(short, long)]
        server: Option<String>,
    },

    /// Stop/Kill a process
    #[command(visible_alias = "kill")]
    Stop {
        id: usize,
        /// Server
        #[arg(short, long)]
        server: Option<String>,
    },

    /// Stop then remove a process
    #[command(visible_alias = "rm")]
    Remove {
        id: usize,
        /// Server
        #[arg(short, long)]
        server: Option<String>,
    },

    /// Get env of a process
    #[command(visible_alias = "cmdline")]
    Env {
        id: usize,
        /// Server
        #[arg(short, long)]
        server: Option<String>,
    },

    /// Get information of a process
    #[command(visible_alias = "info")]
    Details {
        id: usize,
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
        format: String,
        /// Server
        #[arg(short, long)]
        server: Option<String>,
    },

    /// List all processes
    #[command(visible_alias = "ls")]
    List {
        /// Format output
        #[arg(long, default_value_t = string!("default"))]
        format: String,
        /// Server
        #[arg(short, long)]
        server: Option<String>,
    },

    /// Get logs from a process
    Logs {
        id: usize,
        #[arg(long, default_value_t = 15, help = "")]
        lines: usize,
        /// Server
        #[arg(short, long)]
        server: Option<String>,
    },

    /// Daemon management
    #[command(visible_alias = "agent", visible_alias = "bgd")]
    Daemon {
        #[command(subcommand)]
        command: Daemon,
    },

    /// Server management
    #[command(visible_alias = "remote", visible_alias = "srv")]
    Server {
        #[command(subcommand)]
        command: Server,
    },
}

fn main() {
    let cli = Cli::parse();
    let mut env = env_logger::Builder::new();
    let level = cli.verbose.log_level_filter();
    let informer = update_informer::new(registry::Crates, "pmc", env!("CARGO_PKG_VERSION"));

    if let Some(version) = informer.check_version().ok().flatten() {
        println!("{} New version is available: {version}", *pmc::helpers::WARN);
    }

    globals::init();
    env.filter_level(level).init();

    match &cli.command {
        Commands::Start { name, args, watch, server } => cli::start(name, args, watch, &defaults(server)),
        Commands::Stop { id, server } => cli::stop(id, &defaults(server)),
        Commands::Remove { id, server } => cli::remove(id, &defaults(server)),
        Commands::Env { id, server } => cli::env(id, &defaults(server)),
        Commands::Details { id, format, server } => cli::info(id, format, &defaults(server)),
        Commands::List { format, server } => cli::list(format, &defaults(server)),
        Commands::Logs { id, lines, server } => cli::logs(id, lines, &defaults(server)),

        Commands::Daemon { command } => match command {
            Daemon::Stop => daemon::stop(),
            Daemon::Reset => daemon::reset(),
            Daemon::Health { format } => daemon::health(format),
            Daemon::Restore { api, webui } => daemon::restart(api, webui, level.as_str() != "OFF"),
        },

        Commands::Server { command } => match command {
            Server::New => cli::server::new(),
            Server::Remove { name } => cli::server::remove(name),
            Server::Default { name } => cli::server::default(name),
            Server::List { format } => cli::server::list(format, cli.verbose.log_level()),
        },
    };

    if !matches!(&cli.command, Commands::Daemon { .. }) && !matches!(&cli.command, Commands::Server { .. }) {
        then!(!daemon::pid::exists(), daemon::restart(&false, &false, false));
    }
}
