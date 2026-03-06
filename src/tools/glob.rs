use anyhow::{Context, Result};
use crate::types::{ToolDef, FunctionDef};
use std::collections::HashMap;
use std::path::Path;

pub fn definition() -> ToolDef {
    ToolDef {
        tool_type: "function".to_string(),
        function: FunctionDef {
            name: "glob".to_string(),
            description: "Find files matching a glob pattern.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g. \"src/**/*.rs\")"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory (default: current directory)"
                    }
                },
                "required": ["pattern"]
            }),
        },
    }
}

pub async fn execute(args: &HashMap<String, serde_json::Value>, root: &Path) -> Result<String> {
    let pattern = super::get_str(args, "pattern").context("Missing 'pattern' argument")?;
    let base = super::get_str(args, "path")
        .map(|p| super::resolve_path(root, p).to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());

    let full_pattern = if pattern.starts_with('/') {
        pattern.to_string()
    } else {
        format!("{base}/{pattern}")
    };

    let entries = glob::glob(&full_pattern)
        .with_context(|| format!("Invalid glob pattern: {full_pattern}"))?;

    let skip = ["node_modules", ".git", "dist", "target"];
    let mut files: Vec<String> = Vec::new();

    for entry in entries {
        if let Ok(path) = entry {
            let path_str = path.to_string_lossy().to_string();
            if skip.iter().any(|s| path_str.contains(&format!("/{s}/"))) {
                continue;
            }
            files.push(path_str);
            if files.len() >= 200 {
                break;
            }
        }
    }

    files.sort();

    if files.is_empty() {
        Ok(format!("No files match pattern: {pattern}"))
    } else {
        let total = files.len();
        let mut output = files.join("\n");
        if total >= 200 {
            output.push_str(&format!("\n\n... capped at 200 files"));
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_glob_pattern() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();
        std::fs::write(dir.path().join("c.rs"), "").unwrap();

        let mut args = HashMap::new();
        args.insert("pattern".into(), serde_json::json!("*.txt"));
        args.insert("path".into(), serde_json::json!(dir.path().to_str().unwrap()));
        let root = Path::new("/");
        let result = execute(&args, root).await.unwrap();
        assert!(result.contains("a.txt"));
        assert!(result.contains("b.txt"));
        assert!(!result.contains("c.rs"));
    }
}
