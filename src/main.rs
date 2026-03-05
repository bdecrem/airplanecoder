mod self_test;
mod tui;

use airplane::{agent, anthropic, types};
use clap::Parser;

#[derive(Parser)]
#[command(name = "airplane", about = "Offline coding agent powered by local Qwen models + Claude API")]
struct Cli {
    /// Run self-test (check Ollama, tools, etc.)
    #[arg(long)]
    self_test: bool,

    /// Run in REPL mode (no TUI)
    #[arg(long)]
    repl: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.self_test {
        return self_test::run_self_test().await;
    }

    if cli.repl {
        return run_repl().await;
    }

    tui::run_tui().await
}

async fn run_repl() -> anyhow::Result<()> {
    let model = std::env::var("AIRPLANE_MODEL").unwrap_or_else(|_| "qwen3.5:4b".to_string());
    let backend = agent::LlmBackend::new();

    println!("Airplane Coder — REPL mode");
    println!("Model: {model}");
    println!("Type your message (Ctrl+D to quit)\n");

    if !anthropic::is_anthropic_model(&model) && !backend.ollama.is_available().await {
        eprintln!("Warning: Ollama is not running. Start it with: ollama serve");
    }

    let mut messages: Vec<types::Message> = Vec::new();
    let stdin = std::io::stdin();

    loop {
        eprint!("> ");
        let mut input = String::new();
        if stdin.read_line(&mut input)? == 0 {
            break; // EOF
        }
        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }
        if input == "/exit" {
            break;
        }

        messages.push(types::Message {
            role: "user".to_string(),
            content: input,
            tool_calls: None,
            tool_call_id: None,
        });

        agent::run_agent_turn_repl(&backend, &model, &mut messages).await?;
    }

    Ok(())
}
