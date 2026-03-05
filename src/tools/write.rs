use anyhow::{Context, Result};
use crate::types::{ToolDef, FunctionDef};
use std::collections::HashMap;
use std::path::Path;

pub fn definition() -> ToolDef {
    ToolDef {
        tool_type: "function".to_string(),
        function: FunctionDef {
            name: "write_file".to_string(),
            description: "Create or overwrite a file. Creates parent directories if needed.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "File content"
                    }
                },
                "required": ["path", "content"]
            }),
        },
    }
}

pub async fn execute(args: &HashMap<String, serde_json::Value>) -> Result<String> {
    let path = super::get_str(args, "path").context("Missing 'path' argument")?;
    let content = super::get_str(args, "content").context("Missing 'content' argument")?;

    if let Some(parent) = Path::new(path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Cannot create directory: {}", parent.display()))?;
    }

    tokio::fs::write(path, content)
        .await
        .with_context(|| format!("Cannot write file: {path}"))?;

    let line_count = content.lines().count();
    Ok(format!("Wrote {line_count} lines to {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");

        let mut args = HashMap::new();
        args.insert("path".into(), serde_json::json!(path.to_str().unwrap()));
        args.insert("content".into(), serde_json::json!("hello\nworld\n"));
        let result = execute(&args).await.unwrap();
        assert!(result.contains("2 lines"));

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello\nworld\n");
    }

    #[tokio::test]
    async fn test_write_creates_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/test.txt");

        let mut args = HashMap::new();
        args.insert("path".into(), serde_json::json!(path.to_str().unwrap()));
        args.insert("content".into(), serde_json::json!("nested"));
        let result = execute(&args).await.unwrap();
        assert!(result.contains("Wrote"));
        assert!(path.exists());
    }
}
