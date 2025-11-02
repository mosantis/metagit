use anyhow::{anyhow, Result};
use colored::*;
use std::collections::HashMap;

use std::path::Path;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::models::Config;
use crate::utils::{execute_script, ScriptType};

#[derive(Clone)]
enum TaskStatus {
    Waiting,
    Running,
    Completed,
    Failed(String),
}

pub fn run_command(task_name: &str) -> Result<()> {
    let config = Config::load(".mgit_config.json")?;

    // Find the task
    let task = config
        .tasks
        .iter()
        .find(|t| t.name == task_name)
        .ok_or_else(|| anyhow!("Task '{}' not found", task_name))?;

    println!("Executing \"{}\"...\n", task_name.cyan().bold());

    // Create a shared status map
    let status_map: Arc<Mutex<HashMap<String, TaskStatus>>> = Arc::new(Mutex::new(HashMap::new()));

    // Initialize all tasks as waiting
    for step in &task.steps {
        let key = format!("{}:{}", step.repo, step.cmd);
        status_map.lock().unwrap().insert(key, TaskStatus::Waiting);
    }

    // Spawn a thread to display progress
    let status_map_display = status_map.clone();
    let steps = task.steps.clone();
    let task_name_owned = task_name.to_string();
    let display_handle = thread::spawn(move || {
        loop {
            // Clear screen and move cursor to top (simple version)
            print!("\x1B[2J\x1B[1;1H");
            println!("Executing \"{}\"...\n", task_name_owned.cyan().bold());

            // Clone the status map to avoid holding the lock during printing
            let statuses_snapshot: HashMap<String, TaskStatus> = {
                let statuses = status_map_display.lock().unwrap();
                statuses.clone()
            };

            let mut all_done = true;

            for step in &steps {
                let key = format!("{}:{}", step.repo, step.cmd);
                let status = statuses_snapshot.get(&key).unwrap();

                let (status_text, color_fn): (String, fn(&str) -> ColoredString) = match status {
                    TaskStatus::Waiting => {
                        all_done = false;
                        ("waiting...".to_string(), |s| s.yellow())
                    }
                    TaskStatus::Running => {
                        all_done = false;
                        ("running...".to_string(), |s| s.blue())
                    }
                    TaskStatus::Completed => ("completed.".to_string(), |s| s.green()),
                    TaskStatus::Failed(err) => (format!("failed: {}", err), |s| s.red()),
                };

                let args_display = step.args.join(" ");
                let cmd_display = if args_display.is_empty() {
                    step.cmd.clone()
                } else {
                    format!("{} {}", step.cmd, args_display)
                };

                println!(
                    "  {:<20} {:<20} [{}]",
                    step.repo,
                    color_fn(&status_text),
                    cmd_display.dimmed()
                );
            }

            if all_done {
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    // Execute tasks sequentially for now
    for step in &task.steps {
        let key = format!("{}:{}", step.repo, step.cmd);
        let repo_path = Path::new(&step.repo);

        if !repo_path.exists() {
            status_map.lock().unwrap().insert(
                key.clone(),
                TaskStatus::Failed(format!("repository not found")),
            );
            continue;
        }

        // Update status to running
        status_map
            .lock()
            .unwrap()
            .insert(key.clone(), TaskStatus::Running);

        // Determine script type
        let script_type = if step.step_type == "sh" {
            ScriptType::Shell
        } else {
            ScriptType::from_path(&step.cmd)
        };

        // Execute
        match execute_script(script_type, &step.cmd, &step.args, repo_path) {
            Ok(child) => {
                // Use wait_with_output() to properly handle stdout/stderr and avoid deadlocks
                match child.wait_with_output() {
                    Ok(output) => {
                        if output.status.success() {
                            status_map
                                .lock()
                                .unwrap()
                                .insert(key.clone(), TaskStatus::Completed);
                        } else {
                            status_map.lock().unwrap().insert(
                                key.clone(),
                                TaskStatus::Failed(format!(
                                    "exit code {}",
                                    output.status.code().unwrap_or(-1)
                                )),
                            );
                        }
                    }
                    Err(e) => {
                        status_map
                            .lock()
                            .unwrap()
                            .insert(key.clone(), TaskStatus::Failed(e.to_string()));
                    }
                }
            }
            Err(e) => {
                status_map
                    .lock()
                    .unwrap()
                    .insert(key.clone(), TaskStatus::Failed(e.to_string()));
            }
        }
    }

    // Wait for display thread to finish
    display_handle.join().unwrap();

    println!("\nTask execution complete.");

    Ok(())
}
