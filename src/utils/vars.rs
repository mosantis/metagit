use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::env;

/// Variable context that holds all available variables for substitution
#[derive(Debug)]
pub struct VarContext {
    vars: HashMap<String, String>,
}

impl VarContext {
    /// Create a new variable context with:
    /// - All current environment variables
    /// - Predefined variables (CWD, PROJECT_DIR, HOME)
    /// - User-defined variables from -D flags
    pub fn new(
        project_dir: &std::path::Path,
        user_defines: Vec<String>,
    ) -> Result<Self> {
        let mut vars = HashMap::new();

        // Add all environment variables
        for (key, value) in env::vars() {
            vars.insert(key, value);
        }

        // Add predefined variables
        // CWD - current working directory where mgit was invoked
        if let Ok(cwd) = env::current_dir() {
            vars.insert("CWD".to_string(), cwd.to_string_lossy().to_string());
        }

        // PROJECT_DIR - location of .mgitconfig.json
        vars.insert(
            "PROJECT_DIR".to_string(),
            project_dir.to_string_lossy().to_string(),
        );

        // HOME - user's home directory (also available from env, but ensure it's set)
        if let Some(home) = dirs::home_dir() {
            vars.insert("HOME".to_string(), home.to_string_lossy().to_string());
        }

        // Parse user-defined variables from -D flags
        for define in user_defines {
            let parts: Vec<&str> = define.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(anyhow!(
                    "Invalid variable definition '{}'. Expected format: VAR=VALUE",
                    define
                ));
            }
            vars.insert(parts[0].to_string(), parts[1].to_string());
        }

        Ok(Self { vars })
    }

    /// Substitute variables in a string
    /// Supports both $(VAR) and ${VAR} syntax
    /// Also handles tilde (~) expansion at the beginning of paths
    pub fn substitute(&self, input: &str) -> Result<String> {
        let mut result = input.to_string();

        // Handle tilde expansion first (only at the beginning)
        if result.starts_with("~/") || result == "~" {
            if let Some(home) = self.vars.get("HOME") {
                result = result.replacen("~", home, 1);
            }
        }

        // Substitute $(VAR) and ${VAR} patterns
        // We'll do this iteratively to handle nested cases properly
        let mut changed = true;
        let max_iterations = 10; // Prevent infinite loops
        let mut iteration = 0;

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            // Pattern 1: $(VAR)
            result = self.substitute_pattern(&result, "$(", ")", &mut changed)?;

            // Pattern 2: ${VAR}
            result = self.substitute_pattern(&result, "${", "}", &mut changed)?;
        }

        Ok(result)
    }

    /// Helper function to substitute a specific pattern (either $(...) or ${...})
    fn substitute_pattern(
        &self,
        input: &str,
        start_marker: &str,
        end_marker: &str,
        changed: &mut bool,
    ) -> Result<String> {
        let mut result = String::new();
        let mut remaining = input;

        while let Some(start_pos) = remaining.find(start_marker) {
            // Add everything before the marker
            result.push_str(&remaining[..start_pos]);

            // Find the closing marker
            let after_marker = &remaining[start_pos + start_marker.len()..];
            if let Some(end_pos) = after_marker.find(end_marker) {
                let var_name = &after_marker[..end_pos];

                // Look up the variable
                if let Some(value) = self.vars.get(var_name) {
                    result.push_str(value);
                    *changed = true;
                } else {
                    return Err(anyhow!(
                        "Undefined variable: {}{}{}",
                        start_marker,
                        var_name,
                        end_marker
                    ));
                }

                // Move past the closing marker
                remaining = &after_marker[end_pos + end_marker.len()..];
            } else {
                return Err(anyhow!(
                    "Unclosed variable reference: {}",
                    start_marker
                ));
            }
        }

        // Add any remaining text
        result.push_str(remaining);

        Ok(result)
    }

    /// Get the raw variable value (for debugging/testing)
    #[allow(dead_code)]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.vars.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_basic_substitution() {
        let project_dir = Path::new("/project");
        let ctx = VarContext::new(project_dir, vec![
            "VAR1=value1".to_string(),
            "VAR2=value2".to_string(),
        ])
        .unwrap();

        assert_eq!(ctx.substitute("$(VAR1)").unwrap(), "value1");
        assert_eq!(ctx.substitute("${VAR2}").unwrap(), "value2");
        assert_eq!(
            ctx.substitute("prefix_$(VAR1)_suffix").unwrap(),
            "prefix_value1_suffix"
        );
    }

    #[test]
    fn test_predefined_vars() {
        let project_dir = Path::new("/project");
        let ctx = VarContext::new(project_dir, vec![]).unwrap();

        let result = ctx.substitute("$(PROJECT_DIR)").unwrap();
        assert!(result.contains("project"));

        let result = ctx.substitute("$(CWD)").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_tilde_expansion() {
        let project_dir = Path::new("/project");
        let ctx = VarContext::new(project_dir, vec![]).unwrap();

        let result = ctx.substitute("~/Documents").unwrap();
        assert!(!result.starts_with("~"));
        assert!(result.contains("Documents"));
    }

    #[test]
    fn test_mixed_syntax() {
        let project_dir = Path::new("/project");
        let ctx = VarContext::new(project_dir, vec![
            "A=hello".to_string(),
            "B=world".to_string(),
        ])
        .unwrap();

        assert_eq!(ctx.substitute("$(A) ${B}").unwrap(), "hello world");
    }

    #[test]
    fn test_undefined_variable() {
        let project_dir = Path::new("/project");
        let ctx = VarContext::new(project_dir, vec![]).unwrap();

        let result = ctx.substitute("$(UNDEFINED_VAR)");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Undefined variable"));
    }

    #[test]
    fn test_unclosed_variable() {
        let project_dir = Path::new("/project");
        let ctx = VarContext::new(project_dir, vec![]).unwrap();

        let result = ctx.substitute("$(UNCLOSED");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unclosed variable"));
    }

    #[test]
    fn test_invalid_define_format() {
        let project_dir = Path::new("/project");
        let result = VarContext::new(project_dir, vec!["INVALID".to_string()]);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid variable definition"));
    }

    #[test]
    fn test_env_vars() {
        let project_dir = Path::new("/project");
        // Set a test environment variable
        env::set_var("TEST_VAR_12345", "test_value");

        let ctx = VarContext::new(project_dir, vec![]).unwrap();
        assert_eq!(ctx.substitute("$(TEST_VAR_12345)").unwrap(), "test_value");

        // Clean up
        env::remove_var("TEST_VAR_12345");
    }
}
