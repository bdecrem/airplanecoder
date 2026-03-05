use anyhow::{Context, Result};
use crate::types::{ToolDef, FunctionDef};
use std::collections::HashMap;

pub fn definition() -> ToolDef {
    ToolDef {
        tool_type: "function".to_string(),
        function: FunctionDef {
            name: "read_file".to_string(),
            description: "Read file contents with line numbers".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to read"
                    },
                    "offset": {
                        "type": "number",
                        "description": "Start line (1-based, default: 1)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Max lines to read"
                    }
                },
                "required": ["path"]
            }),
        },
    }
}

pub async fn execute(args: &HashMap<String, serde_json::Value>) -> Result<String> {
    let path = super::get_str(args, "path").context("Missing 'path' argument")?;
    let offset = super::get_u64(args, "offset").unwrap_or(1).max(1) as usize;
    let limit = super::get_u64(args, "limit").map(|l| l as usize);

    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Cannot read file: {path}"))?;

    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let start = (offset - 1).min(total);
    let end = match limit {
        Some(l) => (start + l).min(total),
        None => total,
    };

    let mut output = String::new();
    for (i, line) in lines[start..end].iter().enumerate() {
        let line_num = start + i + 1;
        output.push_str(&format!("{line_num:5} | {line}\n"));
    }

    if output.is_empty() {
        Ok(format!("File {path} is empty ({total} lines total)"))
    } else {
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_read_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "line one").unwrap();
        writeln!(tmp, "line two").unwrap();
        writeln!(tmp, "line three").unwrap();

        let mut args = HashMap::new();
        args.insert("path".into(), serde_json::json!(tmp.path().to_str().unwrap()));
        let result = execute(&args).await.unwrap();
        assert!(result.contains("line one"));
        assert!(result.contains("line three"));
    }

    #[tokio::test]
    async fn test_read_with_offset_limit() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        for i in 1..=10 {
            writeln!(tmp, "line {i}").unwrap();
        }

        let mut args = HashMap::new();
        args.insert("path".into(), serde_json::json!(tmp.path().to_str().unwrap()));
        args.insert("offset".into(), serde_json::json!(3));
        args.insert("limit".into(), serde_json::json!(2));
        let result = execute(&args).await.unwrap();
        assert!(result.contains("line 3"));
        assert!(result.contains("line 4"));
        assert!(!result.contains("line 5"));
    }
}
