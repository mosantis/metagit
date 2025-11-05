use anyhow::Result;
use colored::*;

use crate::models::Config;
use crate::utils::pull_repo;

pub fn pull_command(debug: bool) -> Result<()> {
    let config = Config::load_from_project()?;

    if debug {
        println!("{}", "🔍 DEBUG MODE ENABLED".bright_cyan().bold());
        println!();
    }

    println!("Pulling repositories...\n");

    for repo_config in &config.repositories {
        let repo_path = config.resolve_repo_path(&repo_config.name);

        if !repo_path.exists() {
            println!("{:<30} {}",repo_config.name.yellow(), "not found".red());
            continue;
        }

        if debug {
            println!("{}", repo_config.name);
        } else {
            print!("{:<30} ", repo_config.name);
        }
        match pull_repo(&repo_path, debug) {
            Ok(msg) => println!("{}", msg.green()),
            Err(e) => println!("{}: {}", "failed".red(), e),
        }
    }

    Ok(())
}
