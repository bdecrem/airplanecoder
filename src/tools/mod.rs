pub mod read;
pub mod write;
pub mod edit;
pub mod shell;
pub mod grep;
pub mod glob;

use crate::types::ToolDef;
use std::collections::HashMap;

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

pub async fn execute_tool(name: &str, args: &HashMap<String, serde_json::Value>) -> String {
    let result = match name {
        "read_file" => read::execute(args).await,
        "write_file" => write::execute(args).await,
        "edit_file" => edit::execute(args).await,
        "shell" => shell::execute(args).await,
        "grep" => grep::execute(args).await,
        "glob" => glob::execute(args).await,
        _ => Err(anyhow::anyhow!("Unknown tool: {name}")),
    };
    match result {
        Ok(output) => output,
        Err(e) => format!("Error: {e}"),
    }
}

fn get_str<'a>(args: &'a HashMap<String, serde_json::Value>, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn get_u64(args: &HashMap<String, serde_json::Value>, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| v.as_u64())
}
