use anyhow::{Context, Result};
use crate::types::{ToolDef, FunctionDef};
use std::collections::HashMap;

pub fn definition() -> ToolDef {
    ToolDef {
        tool_type: "function".to_string(),
        function: FunctionDef {
            name: "edit_file".to_string(),
            description: "Search-and-replace in a file. The old_string must match exactly once.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact string to find (must match exactly once)"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement string"
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        },
    }
}

pub async fn execute(args: &HashMap<String, serde_json::Value>) -> Result<String> {
    let path = super::get_str(args, "path").context("Missing 'path' argument")?;
    let old_string = super::get_str(args, "old_string").context("Missing 'old_string' argument")?;
    let new_string = super::get_str(args, "new_string").context("Missing 'new_string' argument")?;

    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Cannot read file: {path}"))?;

    let count = content.matches(old_string).count();
    if count == 0 {
        anyhow::bail!("old_string not found in {path}");
    }
    if count > 1 {
        anyhow::bail!("old_string matches {count} times in {path} (must match exactly once)");
    }

    let new_content = content.replacen(old_string, new_string, 1);
    tokio::fs::write(path, &new_content)
        .await
        .with_context(|| format!("Cannot write file: {path}"))?;

    Ok(format!("Edited {path} (replaced 1 occurrence)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_edit_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "hello world").unwrap();

        let mut args = HashMap::new();
        args.insert("path".into(), serde_json::json!(tmp.path().to_str().unwrap()));
        args.insert("old_string".into(), serde_json::json!("world"));
        args.insert("new_string".into(), serde_json::json!("rust"));
        let result = execute(&args).await.unwrap();
        assert!(result.contains("replaced 1"));

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(content, "hello rust");
    }

    #[tokio::test]
    async fn test_edit_no_match() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "hello world").unwrap();

        let mut args = HashMap::new();
        args.insert("path".into(), serde_json::json!(tmp.path().to_str().unwrap()));
        args.insert("old_string".into(), serde_json::json!("xyz"));
        args.insert("new_string".into(), serde_json::json!("abc"));
        let result = execute(&args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_edit_multiple_matches() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "aaa aaa aaa").unwrap();

        let mut args = HashMap::new();
        args.insert("path".into(), serde_json::json!(tmp.path().to_str().unwrap()));
        args.insert("old_string".into(), serde_json::json!("aaa"));
        args.insert("new_string".into(), serde_json::json!("bbb"));
        let result = execute(&args).await;
        assert!(result.is_err());
    }
}
