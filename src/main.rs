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
    /// Initialize .mgitconfig.json by scanning current directory
    Init,

    /// Show status of all repositories
    Status {
        /// Show detailed status including all branches
        #[arg(short, long)]
        detailed: bool,
    },

    /// Pull all repositories
    Pull,

    /// Push all repositories
    Push,

    /// Sync (pull & push) all repositories
    Sync,

    /// Run a task defined in .mgitconfig.json (run without task name to list available tasks)
    Run {
        /// Name of the task to run (optional - omit to list all tasks)
        task_name: Option<String>,

        /// Show detailed task information
        #[arg(short, long)]
        detailed: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init_command()?,
        Commands::Status { detailed } => status_command(detailed)?,
        Commands::Pull => pull_command()?,
        Commands::Push => push_command()?,
        Commands::Sync => sync_command()?,
        Commands::Run { task_name, detailed } => run_command(task_name.as_deref(), detailed)?,
    }

    Ok(())
}
