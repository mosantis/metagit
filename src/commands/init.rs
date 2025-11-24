use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::commands::refresh_command;
use crate::models::{Config, Repository};
use crate::utils::{get_repo_url, is_git_repo};

pub fn init_command() -> Result<()> {
    let config_path = ".mgitconfig.yaml";

    if Path::new(config_path).exists() {
        println!("Configuration file already exists at {}", config_path);
        return Ok(());
    }

    println!("Scanning current directory for git repositories...");

    let mut repositories = Vec::new();

    // Walk through immediate subdirectories
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if is_git_repo(&path) {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                if let Ok(url) = get_repo_url(&path) {
                    println!("  Found repository: {} ({})", name, url);
                    repositories.push(Repository { name, url });
                }
            }
        }
    }

    if repositories.is_empty() {
        println!("No git repositories found in current directory.");
        println!("Creating empty configuration file...");
    } else {
        println!(
            "\nFound {} repositor{}.",
            repositories.len(),
            if repositories.len() == 1 { "y" } else { "ies" }
        );
    }

    let config = Config {
        repositories,
        tasks: Vec::new(),
        shells: Default::default(),
        credentials: HashMap::new(),
        users: HashMap::new(),
        tags: HashMap::new(),
        config_dir: None,
    };

    config.save(config_path)?;
    println!("Configuration saved to {}", config_path);

    // Automatically refresh repository states if we found any repositories
    if !config.repositories.is_empty() {
        println!();
        refresh_command()?;
    }

    Ok(())
}
