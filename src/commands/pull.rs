use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::models::Config;
use crate::utils::pull_repo;

pub fn pull_command() -> Result<()> {
    let config = Config::load(".mgitconfig.json")?;

    println!("Pulling repositories...\n");

    for repo_config in &config.repositories {
        let repo_path = Path::new(&repo_config.name);

        if !repo_path.exists() {
            println!("{:<30} {}",repo_config.name.yellow(), "not found".red());
            continue;
        }

        print!("{:<30} ", repo_config.name);
        match pull_repo(repo_path) {
            Ok(msg) => println!("{}", msg.green()),
            Err(e) => println!("{}: {}", "failed".red(), e),
        }
    }

    Ok(())
}
