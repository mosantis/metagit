use anyhow::{anyhow, Result};
use colored::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::models::Config;
use crate::utils::{execute_script, icons, ScriptType};

#[derive(Clone)]
enum TaskStatus {
    Waiting,
    Running,
    Completed,
    Failed(String, Option<(String, String)>), // (error_type, (stdout, stderr))
}

impl TaskStatus {
    fn format_error_message(error_type: &str, exit_code: i32) -> String {
        match error_type {
            "not_found" => format!("❌ script not found!"),
            "exit_code" => format!("❌ script execution failed! (errcode: {})", exit_code),
            _ => format!("❌ {}", error_type),
        }
    }
}

pub fn run_command(task_name: &str) -> Result<()> {
    let config = Config::load(".mgitconfig.json")?;

    // Find the task
    let task = config
        .tasks
        .iter()
        .find(|t| t.name == task_name)
        .ok_or_else(|| anyhow!("Task '{}' not found", task_name))?;

    println!("Executing \"{}\"...\n", task_name.cyan().bold());

    // Create a shared status map
    let status_map: Arc<Mutex<HashMap<String, TaskStatus>>> = Arc::new(Mutex::new(HashMap::new()));

    // Filter steps to only those that match the current platform
    let steps_to_run: Vec<_> = task
        .steps
        .iter()
        .filter(|step| step.should_run_on_current_platform())
        .cloned()
        .collect();

    // Initialize all tasks as waiting (only for steps that will run)
    for step in &steps_to_run {
        let key = format!("{}:{}", step.repo, step.cmd);
        status_map.lock().unwrap().insert(key, TaskStatus::Waiting);
    }

    // Spawn a thread to display progress
    let status_map_display = status_map.clone();
    let steps = steps_to_run.clone();
    let task_name_owned = task_name.to_string();
    let display_handle = thread::spawn(move || {
        let mut first_render = true;
        let mut previous_statuses: HashMap<String, TaskStatus> = HashMap::new();

        loop {
            // Clone the status map to avoid holding the lock during printing
            let statuses_snapshot: HashMap<String, TaskStatus> = {
                let statuses = status_map_display.lock().unwrap();
                statuses.clone()
            };

            if first_render {
                // On first render, print the header and all lines
                println!("Executing \"{}\"...\n", task_name_owned.cyan().bold());

                for step in &steps {
                    let key = format!("{}:{}", step.repo, step.cmd);
                    let status = statuses_snapshot.get(&key).unwrap();

                    let args_display = step.args.join(" ");
                    let cmd_display = if args_display.is_empty() {
                        step.cmd.clone()
                    } else {
                        format!("{} {}", step.cmd, args_display)
                    };

                    let line = match status {
                        TaskStatus::Waiting => {
                            format!(
                                "  {:<20} {:<30} {}",
                                step.repo,
                                format!("{} waiting...", icons::status::waiting()).yellow(),
                                cmd_display.dimmed()
                            )
                        }
                        TaskStatus::Running => {
                            format!(
                                "  {:<20} {:<30} {}",
                                step.repo,
                                format!("{} running...", icons::status::running()).blue(),
                                cmd_display.dimmed()
                            )
                        }
                        TaskStatus::Completed => {
                            format!(
                                "  {:<20} {:<30} {}",
                                step.repo,
                                format!("{} completed", icons::status::success()).green(),
                                cmd_display.dimmed()
                            )
                        }
                        TaskStatus::Failed(err, _) => {
                            format!(
                                "  {:<20} {:<30} {}",
                                step.repo,
                                cmd_display.dimmed(),
                                err.red()
                            )
                        }
                    };

                    println!("{}", line);
                }

                previous_statuses = statuses_snapshot.clone();
                first_render = false;
            } else {
                // Only update lines that have changed
                let mut lines_to_update: Vec<(usize, String)> = Vec::new();

                for (idx, step) in steps.iter().enumerate() {
                    let key = format!("{}:{}", step.repo, step.cmd);
                    let current_status = statuses_snapshot.get(&key).unwrap();
                    let previous_status = previous_statuses.get(&key);

                    // Check if status has changed
                    let status_changed = match (previous_status, current_status) {
                        (Some(TaskStatus::Waiting), TaskStatus::Waiting) => false,
                        (Some(TaskStatus::Running), TaskStatus::Running) => false,
                        (Some(TaskStatus::Completed), TaskStatus::Completed) => false,
                        (Some(TaskStatus::Failed(prev_err, _)), TaskStatus::Failed(curr_err, _)) => {
                            prev_err != curr_err
                        }
                        _ => true,
                    };

                    if status_changed {
                        let args_display = step.args.join(" ");
                        let cmd_display = if args_display.is_empty() {
                            step.cmd.clone()
                        } else {
                            format!("{} {}", step.cmd, args_display)
                        };

                        let line = match current_status {
                            TaskStatus::Waiting => {
                                format!(
                                    "  {:<20} {:<30} {}",
                                    step.repo,
                                    format!("{} waiting...", icons::status::waiting()).yellow(),
                                    cmd_display.dimmed()
                                )
                            }
                            TaskStatus::Running => {
                                format!(
                                    "  {:<20} {:<30} {}",
                                    step.repo,
                                    format!("{} running...", icons::status::running()).blue(),
                                    cmd_display.dimmed()
                                )
                            }
                            TaskStatus::Completed => {
                                format!(
                                    "  {:<20} {:<30} {}",
                                    step.repo,
                                    format!("{} completed", icons::status::success()).green(),
                                    cmd_display.dimmed()
                                )
                            }
                            TaskStatus::Failed(err, _) => {
                                format!(
                                    "  {:<20} {:<30} {}",
                                    step.repo,
                                    cmd_display.dimmed(),
                                    err.red()
                                )
                            }
                        };

                        lines_to_update.push((idx, line));
                    }
                }

                // Update only changed lines
                if !lines_to_update.is_empty() {
                    for (line_idx, line_content) in &lines_to_update {
                        // Move cursor up to the line that needs updating
                        // We're currently at the bottom, so move up (total_lines - line_idx) times
                        let lines_up = steps.len() - line_idx;
                        print!("\x1B[{}A", lines_up);
                        // Move to start of line and clear it
                        print!("\r\x1B[K");
                        // Print the new content
                        print!("{}", line_content);
                        // Move cursor back down to bottom
                        print!("\x1B[{}B", lines_up);
                    }

                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }

                previous_statuses = statuses_snapshot.clone();
            }

            let mut all_done = true;
            for step in &steps {
                let key = format!("{}:{}", step.repo, step.cmd);
                let status = statuses_snapshot.get(&key).unwrap();
                if !matches!(status, TaskStatus::Completed | TaskStatus::Failed(_, _)) {
                    all_done = false;
                    break;
                }
            }

            if all_done {
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    // Execute tasks sequentially for now
    for step in &steps_to_run {
        let key = format!("{}:{}", step.repo, step.cmd);
        let repo_path = Path::new(&step.repo);

        if !repo_path.exists() {
            status_map.lock().unwrap().insert(
                key.clone(),
                TaskStatus::Failed("repository not found".to_string(), None),
            );
            continue;
        }

        // Update status to running
        status_map
            .lock()
            .unwrap()
            .insert(key.clone(), TaskStatus::Running);

        // Determine script type
        // Priority: explicit type > inferred from extension
        let script_type = if !step.step_type.is_empty() {
            // Explicit type specified
            match step.step_type.as_str() {
                "sh" => ScriptType::Shell,
                "bat" | "cmd" => ScriptType::Batch,
                "ps1" => ScriptType::PowerShell,
                "exe" => ScriptType::Executable,
                _ => ScriptType::from_path(&step.cmd), // Unknown type, try to infer
            }
        } else {
            // No explicit type, infer from file extension
            ScriptType::from_path(&step.cmd)
        };

        // Execute
        match execute_script(script_type, &step.cmd, &step.args, repo_path, &config.shells) {
            Ok(child) => {
                // Use wait_with_output() to properly handle stdout/stderr and avoid deadlocks
                match child.wait_with_output() {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                        if output.status.success() {
                            status_map
                                .lock()
                                .unwrap()
                                .insert(key.clone(), TaskStatus::Completed);
                        } else {
                            let exit_code = output.status.code().unwrap_or(-1);
                            let error_msg = TaskStatus::format_error_message("exit_code", exit_code);
                            status_map.lock().unwrap().insert(
                                key.clone(),
                                TaskStatus::Failed(error_msg, Some((stdout, stderr))),
                            );
                        }
                    }
                    Err(e) => {
                        let error_msg = if e.to_string().contains("not found") || e.to_string().contains("cannot find") {
                            TaskStatus::format_error_message("not_found", 0)
                        } else {
                            format!("❌ {}", e)
                        };
                        status_map
                            .lock()
                            .unwrap()
                            .insert(key.clone(), TaskStatus::Failed(error_msg, None));
                    }
                }
            }
            Err(e) => {
                let error_msg = if e.to_string().contains("not found") || e.to_string().contains("cannot find") {
                    TaskStatus::format_error_message("not_found", 0)
                } else {
                    format!("❌ {}", e)
                };
                status_map
                    .lock()
                    .unwrap()
                    .insert(key.clone(), TaskStatus::Failed(error_msg, None));
            }
        }
    }

    // Wait for display thread to finish
    display_handle.join().unwrap();

    println!("\nTask execution complete.");

    // Display post-mortem for failed tasks
    let final_statuses = status_map.lock().unwrap();
    let mut has_failures = false;

    for step in &steps_to_run {
        let key = format!("{}:{}", step.repo, step.cmd);
        if let Some(TaskStatus::Failed(_, Some((stdout, stderr)))) = final_statuses.get(&key) {
            if !has_failures {
                println!("\n{}", "═".repeat(80).dimmed());
                println!("{}", "Failed Task Logs".yellow().bold());
                println!("{}", "═".repeat(80).dimmed());
                has_failures = true;
            }

            let args_display = step.args.join(" ");
            let cmd_display = if args_display.is_empty() {
                step.cmd.clone()
            } else {
                format!("{} {}", step.cmd, args_display)
            };

            println!("\n{} {} {}",
                "▶".red().bold(),
                step.repo.cyan().bold(),
                cmd_display.dimmed()
            );
            println!("{}", "─".repeat(80).dimmed());

            if !stdout.trim().is_empty() {
                println!("{}", "Output:".bold());
                for line in stdout.lines().take(20) {
                    println!("  {}", line);
                }
                if stdout.lines().count() > 20 {
                    println!("  {} (truncated)", format!("... {} more lines", stdout.lines().count() - 20).dimmed());
                }
            }

            if !stderr.trim().is_empty() {
                println!("\n{}", "Errors:".red().bold());
                for line in stderr.lines().take(20) {
                    println!("  {}", line.red());
                }
                if stderr.lines().count() > 20 {
                    println!("  {} (truncated)", format!("... {} more lines", stderr.lines().count() - 20).dimmed());
                }
            }
        }
    }

    if has_failures {
        println!("\n{}", "═".repeat(80).dimmed());
    }

    Ok(())
}
