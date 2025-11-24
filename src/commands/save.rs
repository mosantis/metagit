use crate::models::Config;
use crate::utils::icons;
use anyhow::{anyhow, Result};
use colored::*;
use git2::Repository;
use std::collections::HashMap;

pub fn save_command(tag: &str) -> Result<()> {
    // Reserved tags cannot be saved (they're virtual)
    if tag == "master" || tag == "main" {
        return Err(anyhow!(
            "Tag '{}' is reserved and cannot be saved. Reserved tags: 'master', 'main'",
            tag
        ));
    }

    let mut config = Config::load_from_project()?;

    println!(
        "{} Saving current branches to tag '{}'...\n",
        icons::status::info(),
        tag.cyan().bold()
    );

    let mut branches = HashMap::new();
    let mut success_count = 0;
    let mut error_count = 0;

    // Iterate through all repositories and get current branch
    for repo_config in &config.repositories {
        let repo_path = config.resolve_repo_path(&repo_config.name);

        if !repo_path.exists() {
            println!(
                "  {} {} - repository not found",
                icons::status::error(),
                repo_config.name.yellow()
            );
            error_count += 1;
            continue;
        }

        // Open the repository
        match Repository::open(&repo_path) {
            Ok(repo) => {
                // Get the current branch
                match repo.head() {
                    Ok(head) => {
                        if head.is_branch() {
                            let branch_name = head
                                .shorthand()
                                .ok_or_else(|| anyhow!("Could not get branch name"))?;

                            println!(
                                "  {} {} - {}",
                                icons::status::success(),
                                repo_config.name.cyan(),
                                branch_name.green()
                            );

                            branches.insert(repo_config.name.clone(), branch_name.to_string());
                            success_count += 1;
                        } else {
                            println!(
                                "  {} {} - detached HEAD state",
                                icons::status::warning(),
                                repo_config.name.yellow()
                            );
                            error_count += 1;
                        }
                    }
                    Err(e) => {
                        println!(
                            "  {} {} - could not get current branch: {}",
                            icons::status::error(),
                            repo_config.name.yellow(),
                            e
                        );
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                println!(
                    "  {} {} - could not open repository: {}",
                    icons::status::error(),
                    repo_config.name.yellow(),
                    e
                );
                error_count += 1;
            }
        }
    }

    if branches.is_empty() {
        return Err(anyhow!("No branches could be saved"));
    }

    // Save to config
    config.tags.insert(tag.to_string(), branches);

    // Find the project config path to save to
    let config_path = Config::find_project_config()
        .ok_or_else(|| anyhow!("Could not find .mgitconfig.yaml"))?;

    config.save(config_path.to_str().unwrap())?;

    println!();
    println!(
        "{} Tag '{}' saved successfully! ({} repositories, {} errors)",
        icons::status::success(),
        tag.green().bold(),
        success_count,
        error_count
    );

    if error_count > 0 {
        println!(
            "\n{} Some repositories could not be saved. Check the errors above.",
            icons::status::warning()
        );
    }

    Ok(())
}
