use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::db::StateDb;
use crate::models::Config;
use crate::utils::git::refresh_repo_state;
use crate::utils::icons;

pub fn refresh_command() -> Result<()> {
    let config = Config::load(".mgitconfig.json")?;
    let db = StateDb::open(".mgitdb")?;

    let folder_icon = icons::files::folder();
    let check_icon = icons::status::success();

    println!("{}", "Refreshing repository states...".bold());
    println!();

    let mut success_count = 0;
    let mut error_count = 0;

    for repo_config in &config.repositories {
        let repo_path = Path::new(&repo_config.name);

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

        // Get previous state from database for incremental updates
        let previous_state = db.get_repo_state(&repo_config.name).ok().flatten();

        match refresh_repo_state(repo_path, &repo_config.name, previous_state.as_ref()) {
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

    Ok(())
}
