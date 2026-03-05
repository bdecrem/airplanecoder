use anyhow::{Context, Result};
use crate::types::{ToolDef, FunctionDef};
use std::collections::HashMap;
use tokio::process::Command;

pub fn definition() -> ToolDef {
    ToolDef {
        tool_type: "function".to_string(),
        function: FunctionDef {
            name: "shell".to_string(),
            description: "Execute a shell command. Returns stdout and stderr.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory (default: current directory)"
                    }
                },
                "required": ["command"]
            }),
        },
    }
}

pub async fn execute(args: &HashMap<String, serde_json::Value>) -> Result<String> {
    let command = super::get_str(args, "command").context("Missing 'command' argument")?;
    let cwd = super::get_str(args, "cwd")
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string());

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&cwd)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&stderr);
            }
            if result.is_empty() {
                result = format!("Command completed with exit code {}", output.status.code().unwrap_or(-1));
            }
            Ok(result)
        }
        Ok(Err(e)) => Err(e).context(format!("Failed to execute: {command}")),
        Err(_) => anyhow::bail!("Command timed out after 120 seconds: {command}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_echo() {
        let mut args = HashMap::new();
        args.insert("command".into(), serde_json::json!("echo hello"));
        let result = execute(&args).await.unwrap();
        assert_eq!(result.trim(), "hello");
    }

    #[tokio::test]
    async fn test_shell_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let mut args = HashMap::new();
        args.insert("command".into(), serde_json::json!("pwd"));
        args.insert("cwd".into(), serde_json::json!(dir.path().to_str().unwrap()));
        let result = execute(&args).await.unwrap();
        // On macOS /tmp is /private/tmp, so use canonical paths
        let expected = std::fs::canonicalize(dir.path()).unwrap();
        let actual = std::fs::canonicalize(result.trim()).unwrap();
        assert_eq!(actual, expected);
    }
}
