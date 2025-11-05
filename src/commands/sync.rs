use anyhow::Result;
use colored::*;

use crate::models::Config;
use crate::utils::{pull_repo, push_repo};

pub fn sync_command(debug: bool) -> Result<()> {
    let config = Config::load_from_project()?;

    if debug {
        println!("{}", "🔍 DEBUG MODE ENABLED".bright_cyan().bold());
        println!();
    }

    println!("Syncing repositories (pull & push)...\n");

    for repo_config in &config.repositories {
        let repo_path = config.resolve_repo_path(&repo_config.name);

        if !repo_path.exists() {
            println!("{:<30} {}",repo_config.name.yellow(), "not found".red());
            continue;
        }

        print!("{:<30} ", repo_config.name);

        // Pull first
        match pull_repo(&repo_path, debug) {
            Ok(msg) => print!("pull: {} ", msg.green()),
            Err(e) => {
                println!("pull {}: {}", "failed".red(), e);
                continue; // Skip push if pull failed
            }
        }

        // Then push
        match push_repo(&repo_path, debug) {
            Ok(msg) => println!("| push: {}", msg.green()),
            Err(e) => println!("| push {}: {}", "failed".red(), e),
        }
    }

    Ok(())
}
