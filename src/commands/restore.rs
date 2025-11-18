use crate::models::Config;
use crate::utils::icons;
use anyhow::{anyhow, Result};
use colored::*;
use git2::Repository;
use std::collections::HashMap;

pub fn restore_command(tag: &str) -> Result<()> {
    let config = Config::load_from_project()?;

    println!(
        "{} Restoring branches from tag '{}'...\n",
        icons::status::info(),
        tag.cyan().bold()
    );

    // Handle reserved tags 'master' and 'main'
    let branches = if tag == "master" || tag == "main" {
        // For reserved tags, determine the default branch for each repo
        println!(
            "{} Using reserved tag '{}' - will switch to default branch (master/main) for each repository\n",
            icons::status::info(),
            tag.cyan()
        );

        let mut auto_branches = HashMap::new();
        for repo_config in &config.repositories {
            let repo_path = config.resolve_repo_path(&repo_config.name);

            if !repo_path.exists() {
                continue;
            }

            // Open the repository to find the default branch
            match Repository::open(&repo_path) {
                Ok(repo) => {
                    // Try to find master or main branch
                    let default_branch = if repo.find_branch("main", git2::BranchType::Local).is_ok() {
                        "main"
                    } else if repo.find_branch("master", git2::BranchType::Local).is_ok() {
                        "master"
                    } else {
                        // Try to get the default branch from remote
                        if let Ok(_remote) = repo.find_remote("origin") {
                            if let Ok(head) = repo.find_reference("refs/remotes/origin/HEAD") {
                                if let Some(target) = head.symbolic_target() {
                                    if target.contains("main") {
                                        "main"
                                    } else {
                                        "master"
                                    }
                                } else {
                                    "master" // Default fallback
                                }
                            } else {
                                "master" // Default fallback
                            }
                        } else {
                            "master" // Default fallback
                        }
                    };

                    auto_branches.insert(repo_config.name.clone(), default_branch.to_string());
                }
                Err(_) => {
                    // Skip repositories that can't be opened
                    continue;
                }
            }
        }
        auto_branches
    } else {
        // Load saved tag from config
        config
            .tags
            .get(tag)
            .cloned()
            .ok_or_else(|| anyhow!("Tag '{}' not found. Use 'mgit save {}' to create it.", tag, tag))?
    };

    if branches.is_empty() {
        return Err(anyhow!("No branches to restore for tag '{}'", tag));
    }

    let mut success_count = 0;
    let mut error_count = 0;

    // Restore branches for each repository
    for repo_config in &config.repositories {
        let repo_path = config.resolve_repo_path(&repo_config.name);

        // Skip if no branch saved for this repo
        let branch_name = match branches.get(&repo_config.name) {
            Some(name) => name,
            None => {
                println!(
                    "  {} {} - no branch saved in tag",
                    icons::status::warning(),
                    repo_config.name.yellow()
                );
                continue;
            }
        };

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
                // Check if already on the target branch
                if let Ok(head) = repo.head() {
                    if head.is_branch() {
                        if let Some(current_branch) = head.shorthand() {
                            if current_branch == branch_name {
                                println!(
                                    "  {} {} - already on {}",
                                    icons::status::success(),
                                    repo_config.name.cyan(),
                                    branch_name.green()
                                );
                                success_count += 1;
                                continue;
                            }
                        }
                    }
                }

                // Try to checkout the branch
                match checkout_branch(&repo, branch_name) {
                    Ok(_) => {
                        println!(
                            "  {} {} - switched to {}",
                            icons::status::success(),
                            repo_config.name.cyan(),
                            branch_name.green()
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        println!(
                            "  {} {} - failed to checkout {}: {}",
                            icons::status::error(),
                            repo_config.name.yellow(),
                            branch_name,
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

    println!();
    println!(
        "{} Tag '{}' restored! ({} repositories, {} errors)",
        icons::status::success(),
        tag.green().bold(),
        success_count,
        error_count
    );

    if error_count > 0 {
        println!(
            "\n{} Some repositories could not be restored. Check the errors above.",
            icons::status::warning()
        );
    }

    Ok(())
}

/// Checkout a branch in a repository
fn checkout_branch(repo: &Repository, branch_name: &str) -> Result<()> {
    // Find the branch
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .map_err(|e| anyhow!("Branch '{}' not found: {}", branch_name, e))?;

    // Get the reference
    let reference = branch.get();

    // Get the tree
    let tree = reference
        .peel_to_tree()
        .map_err(|e| anyhow!("Could not get tree: {}", e))?;

    // Checkout the tree
    repo.checkout_tree(tree.as_object(), None)
        .map_err(|e| anyhow!("Could not checkout tree: {}", e))?;

    // Set HEAD to the branch
    repo.set_head(reference.name().ok_or_else(|| anyhow!("Could not get reference name"))?)
        .map_err(|e| anyhow!("Could not set HEAD: {}", e))?;

    Ok(())
}
