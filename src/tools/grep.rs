use anyhow::{Context, Result};
use crate::types::{ToolDef, FunctionDef};
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;

pub fn definition() -> ToolDef {
    ToolDef {
        tool_type: "function".to_string(),
        function: FunctionDef {
            name: "grep".to_string(),
            description: "Search file contents with regex. Returns matching lines with file paths and line numbers.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory or file to search (default: current directory)"
                    },
                    "include": {
                        "type": "string",
                        "description": "File glob filter (e.g. \"*.rs\")"
                    }
                },
                "required": ["pattern"]
            }),
        },
    }
}

pub async fn execute(args: &HashMap<String, serde_json::Value>, root: &Path) -> Result<String> {
    let pattern = super::get_str(args, "pattern").context("Missing 'pattern' argument")?;
    let path = super::get_str(args, "path")
        .map(|p| super::resolve_path(root, p))
        .unwrap_or_else(|| root.to_owned());
    let path = path.to_string_lossy();
    let include = super::get_str(args, "include");

    let mut cmd = Command::new("grep");
    cmd.arg("-rn")
        .arg("--color=never")
        .arg("--exclude-dir=node_modules")
        .arg("--exclude-dir=.git")
        .arg("--exclude-dir=dist")
        .arg("--exclude-dir=target");

    if let Some(inc) = include {
        cmd.arg(format!("--include={inc}"));
    }

    cmd.arg(pattern).arg(&*path);

    let output = cmd.output().await.context("Failed to run grep")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let lines: Vec<&str> = stdout.lines().collect();
    let total = lines.len();

    if total == 0 {
        return Ok(format!("No matches found for pattern: {pattern}"));
    }

    const CAP: usize = 100;
    if total > CAP {
        let truncated: String = lines[..CAP].join("\n");
        Ok(format!("{truncated}\n\n... {total} total matches (showing first {CAP})"))
    } else {
        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_grep_pattern() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        let mut f = std::fs::File::create(&file).unwrap();
        writeln!(f, "hello world").unwrap();
        writeln!(f, "goodbye world").unwrap();
        writeln!(f, "hello rust").unwrap();

        let mut args = HashMap::new();
        args.insert("pattern".into(), serde_json::json!("hello"));
        args.insert("path".into(), serde_json::json!(dir.path().to_str().unwrap()));
        let root = Path::new("/");
        let result = execute(&args, root).await.unwrap();
        assert!(result.contains("hello world"));
        assert!(result.contains("hello rust"));
        assert!(!result.contains("goodbye"));
    }
}
