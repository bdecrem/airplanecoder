pub mod read;
pub mod write;
pub mod edit;
pub mod shell;
pub mod grep;
pub mod glob;

use crate::types::ToolDef;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn get_tool_definitions() -> Vec<ToolDef> {
    vec![
        read::definition(),
        write::definition(),
        edit::definition(),
        shell::definition(),
        grep::definition(),
        glob::definition(),
    ]
}

/// Resolve a path argument against the project root.
/// Absolute paths pass through unchanged.
pub fn resolve_path(root: &Path, p: &str) -> PathBuf {
    let p = Path::new(p);
    if p.is_absolute() {
        p.to_owned()
    } else {
        root.join(p)
    }
}

pub async fn execute_tool(
    name: &str,
    args: &HashMap<String, serde_json::Value>,
    root: &Path,
) -> String {
    let result = match name {
        "read_file" => read::execute(args, root).await,
        "write_file" => write::execute(args, root).await,
        "edit_file" => edit::execute(args, root).await,
        "shell" => shell::execute(args, root).await,
        "grep" => grep::execute(args, root).await,
        "glob" => glob::execute(args, root).await,
        _ => Err(anyhow::anyhow!("Unknown tool: {name}")),
    };
    match result {
        Ok(output) => output,
        Err(e) => format!("Error: {e}"),
    }
}

pub(crate) fn get_str<'a>(args: &'a HashMap<String, serde_json::Value>, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

pub(crate) fn get_u64(args: &HashMap<String, serde_json::Value>, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| v.as_u64())
}
