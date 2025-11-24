mod commands;
mod db;
mod models;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};

use commands::*;

#[derive(Parser)]
#[command(name = "mgit")]
#[command(about = "MetaGit - Enhanced git for multiple repositories", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .mgitconfig.yaml by scanning current directory
    Init,

    /// Show status of all repositories
    Status {
        /// Show all branches (not just current branch)
        #[arg(short, long)]
        all: bool,
    },

    /// Pull all repositories
    Pull {
        /// Enable debug output for troubleshooting connection/credential issues
        #[arg(long)]
        debug: bool,
    },

    /// Push all repositories
    Push {
        /// Enable debug output for troubleshooting connection/credential issues
        #[arg(long)]
        debug: bool,
    },

    /// Sync (pull & push) all repositories
    Sync {
        /// Enable debug output for troubleshooting connection/credential issues
        #[arg(long)]
        debug: bool,
    },

    /// Refresh repository states and collect commit statistics
    Refresh,

    /// Save current branches to a tag
    Save {
        /// Name of the tag to save branches to
        tag: String,
    },

    /// Restore branches from a saved tag (use 'master' or 'main' to switch to default branch)
    Restore {
        /// Name of the tag to restore branches from
        tag: String,
    },

    /// Run a task defined in .mgitconfig.yaml (run without task name to list available tasks)
    Run {
        /// Name of the task to run (optional - omit to list all tasks)
        task_name: Option<String>,

        /// Show detailed task information
        #[arg(short, long)]
        detailed: bool,

        /// Define variables for substitution (e.g., -DVAR1=value1 -DVAR2=value2)
        #[arg(short = 'D', value_name = "VAR=VALUE")]
        defines: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init_command()?,
        Commands::Status { all } => status_command(all)?,
        Commands::Pull { debug } => pull_command(debug)?,
        Commands::Push { debug } => push_command(debug)?,
        Commands::Sync { debug } => sync_command(debug)?,
        Commands::Refresh => refresh_command()?,
        Commands::Save { tag } => save_command(&tag)?,
        Commands::Restore { tag } => restore_command(&tag)?,
        Commands::Run { task_name, detailed, defines } => run_command(task_name.as_deref(), detailed, defines)?,
    }

    Ok(())
}
