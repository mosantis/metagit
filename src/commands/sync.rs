use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::models::Config;
use crate::utils::{pull_repo, push_repo};

pub fn sync_command() -> Result<()> {
    let config = Config::load(".mgit_config.json")?;

    println!("Syncing repositories (pull & push)...\n");

    for repo_config in &config.repositories {
        let repo_path = Path::new(&repo_config.name);

        if !repo_path.exists() {
            println!("{:<30} {}",repo_config.name.yellow(), "not found".red());
            continue;
        }

        print!("{:<30} ", repo_config.name);

        // Pull first
        match pull_repo(repo_path) {
            Ok(msg) => print!("pull: {} ", msg.green()),
            Err(e) => {
                println!("pull {}: {}", "failed".red(), e);
                continue; // Skip push if pull failed
            }
        }

        // Then push
        match push_repo(repo_path) {
            Ok(msg) => println!("| push: {}", msg.green()),
            Err(e) => println!("| push {}: {}", "failed".red(), e),
        }
    }

    Ok(())
}
