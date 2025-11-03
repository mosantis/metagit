use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::models::Config;
use crate::utils::push_repo;

pub fn push_command() -> Result<()> {
    let config = Config::load(".mgitconfig.json")?;

    println!("Pushing repositories...\n");

    for repo_config in &config.repositories {
        let repo_path = Path::new(&repo_config.name);

        if !repo_path.exists() {
            println!("{:<30} {}",repo_config.name.yellow(), "not found".red());
            continue;
        }

        print!("{:<30} ", repo_config.name);
        match push_repo(repo_path) {
            Ok(msg) => println!("{}", msg.green()),
            Err(e) => println!("{}: {}", "failed".red(), e),
        }
    }

    Ok(())
}
