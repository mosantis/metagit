use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;

use crate::db::StateDb;
use crate::models::Config;
use crate::utils::git::{collect_all_author_identities, refresh_repo_state, repair_repository, AuthorIdentity};
use crate::utils::icons;

pub fn refresh_command() -> Result<()> {
    let mut config = Config::load_from_project()?;
    let db = StateDb::open(".mgitdb")?;

    let folder_icon = icons::files::folder();
    let check_icon = icons::status::success();

    println!("{}", "Refreshing repository states...".bold());
    println!();

    let mut success_count = 0;
    let mut error_count = 0;
    let mut repair_count = 0;
    let mut all_identities = HashSet::new();

    for repo_config in &config.repositories {
        let repo_path = config.resolve_repo_path(&repo_config.name);

        if !repo_path.exists() {
            eprintln!(
                "  {} {} - {}",
                folder_icon,
                repo_config.name.yellow(),
                "not found".red()
            );
            error_count += 1;
            continue;
        }

        // Attempt to repair repository before refreshing
        match repair_repository(&repo_path) {
            Ok(repair_result) => {
                if repair_result.has_fixes() {
                    repair_count += 1;

                    // Report what was fixed
                    if repair_result.fixed_fetch_head {
                        println!(
                            "  {} {} - {}",
                            icons::status::info(),
                            repo_config.name.cyan(),
                            "repaired corrupted FETCH_HEAD".yellow()
                        );
                    }

                    for ref_path in &repair_result.removed_corrupted_refs {
                        println!(
                            "  {} {} - {}",
                            icons::status::info(),
                            repo_config.name.cyan(),
                            format!("removed corrupted ref: {}", ref_path).yellow()
                        );
                    }
                }
            }
            Err(e) => {
                // Non-fatal - continue with refresh
                eprintln!(
                    "  {} {} - {}",
                    icons::status::warning(),
                    repo_config.name.yellow(),
                    format!("repair check failed: {}", e).yellow()
                );
            }
        }

        // Collect author identities from this repository
        if let Ok(identities) = collect_all_author_identities(&repo_path) {
            all_identities.extend(identities);
        }

        // Get previous state from database for incremental updates
        let previous_state = db.get_repo_state(&repo_config.name).ok().flatten();

        match refresh_repo_state(&repo_path, &repo_config.name, previous_state.as_ref(), &config.users) {
            Ok(state) => {
                // Save to database
                db.save_repo_state(&state)?;

                let branch_count = state.branches.len();
                let total_commits: usize = state
                    .branches
                    .iter()
                    .flat_map(|b| b.commit_stats.values())
                    .sum();

                println!(
                    "  {} {} {:<30} {} branches, {} commits analyzed",
                    check_icon,
                    folder_icon,
                    repo_config.name.green(),
                    branch_count,
                    total_commits
                );
                success_count += 1;
            }
            Err(e) => {
                eprintln!(
                    "  {} {} - {}",
                    folder_icon,
                    repo_config.name.yellow(),
                    format!("error: {}", e).red()
                );
                error_count += 1;
            }
        }
    }

    // Process author identities - add all identities and track what was actually added
    let mut unmapped_count = 0;
    let mut unmapped_identities: Vec<AuthorIdentity> = all_identities.into_iter().collect();

    // Sort by name for consistent ordering
    unmapped_identities.sort_by(|a, b| a.name.cmp(&b.name));

    for identity in &unmapped_identities {
        if config.add_unmapped_authors(identity.name.clone(), identity.email.clone()) {
            unmapped_count += 1;
        }
    }

    // Save updated config if anything was added
    if unmapped_count > 0 {
        config.save(".mgitconfig.json")?;
    }

    println!();
    if error_count == 0 {
        println!(
            "{}",
            format!("Successfully refreshed {} repositories", success_count)
                .green()
                .bold()
        );
    } else {
        println!(
            "{}",
            format!(
                "Refreshed {} repositories ({} errors)",
                success_count, error_count
            )
            .yellow()
            .bold()
        );
    }

    if repair_count > 0 {
        println!(
            "{}",
            format!(
                "Repaired {} repositor{}",
                repair_count,
                if repair_count == 1 { "y" } else { "ies" }
            )
            .yellow()
        );
    }

    if unmapped_count > 0 {
        println!(
            "{}",
            format!("Added {} new author alias{} to .mgitconfig.json",
                unmapped_count,
                if unmapped_count == 1 { "" } else { "es" }
            )
            .cyan()
        );
    }

    Ok(())
}
