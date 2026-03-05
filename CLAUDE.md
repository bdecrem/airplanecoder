# Airplane Coder

Offline coding agent powered by local Qwen models via Ollama. Single Rust binary.

## Quick Start

```bash
ollama pull qwen3.5:4b           # Get a model
ollama serve                     # Start Ollama
cargo run                        # Launch TUI
cargo run -- --repl              # REPL mode (no TUI)
cargo run -- --self-test         # Run self-test
```

Set model via env: `AIRPLANE_MODEL=qwen3.5:0.8b cargo run`

## Architecture

```
src/
├── main.rs              # CLI (clap): --self-test, --repl flags, launches TUI
├── types.rs             # Message, ToolCall, ToolDef, ChatRequest/Response
├── ollama.rs            # OllamaClient: POST /api/chat, GET /api/tags
├── agent.rs             # Agent loop: messages -> Ollama -> tool calls -> execute -> loop (max 20)
├── self_test.rs         # --self-test: connectivity, parsing, tool smoke tests
├── tui/
│   ├── mod.rs           # App state, async event loop (crossterm poll + agent channel)
│   └── widgets.rs       # Message pane, input bar, status bar, splash screen
└── tools/
    ├── mod.rs           # execute_tool() dispatch, get_tool_definitions()
    ├── read.rs          # read_file — line-numbered output, offset/limit
    ├── write.rs         # write_file — create dirs, overwrite
    ├── edit.rs          # edit_file — search-and-replace, exactly-once match
    ├── shell.rs         # shell — 120s timeout (for cargo build)
    ├── grep.rs          # grep — regex search, 100 result cap
    └── glob.rs          # glob — file patterns, 200 file cap
```

### How the Agent Loop Works

1. User message -> added to conversation
2. Send conversation + tool definitions to Ollama
3. If model returns tool_calls -> execute each tool, append results, loop back to step 2
4. If model returns text only -> display to user, done
5. Max 20 iterations per turn (safety limit)

### Context Management

- Tool results truncated beyond 2000 chars (first/last halves with "... N chars truncated ...")
- Old messages dropped when conversation exceeds 40 messages (keeps system prompt + recent turns)
- If `AIRPLANE.md` exists in cwd, it's prepended to the system prompt

### LLM Client

Direct HTTP to Ollama (`localhost:11434`). Override with `OLLAMA_HOST` env var. Uses blocking mode (`stream: false`) for reliable tool call parsing. Temperature 0.1, context window 8192.

## Tools

Each tool module exports `definition()` -> `ToolDef` and `execute()` -> `Result<String>`. Tool dispatch is an enum match in `tools/mod.rs`.

To add a new tool:
1. Create `src/tools/yourtool.rs` with `definition()` and `execute()`
2. Add module to `src/tools/mod.rs` and wire into `get_tool_definitions()` + `execute_tool()`

## TUI

Ratatui + crossterm. Async event loop polls crossterm events (50ms) + drains agent mpsc channel.

- Scrollable message pane (Shift+Up/Down 3 lines, PageUp/PageDown full page)
- Status bar: current model + working directory
- Ctrl+C cancels current agent turn (second Ctrl+C quits)
- Slash commands: `/model`, `/clear`, `/help`, `/exit`

## Target Models

Default: `qwen3.5:4b`. Override with `AIRPLANE_MODEL` env var.

| Model | Size | Use Case |
|-------|------|----------|
| `qwen3.5:0.8b` | ~0.5GB | Tiny, fast, edge device |
| `qwen3.5:2b` | ~1.5GB | Lightweight, good balance |
| `qwen3.5:4b` | ~2.5GB | Strongest small model |

## Code Practices

- Rust, single binary via `cargo build`
- Async with tokio (reqwest needs it)
- No external API keys or cloud services — everything local
- Tools return strings (success message or error)
- `cargo test` for unit tests, `--self-test` for integration tests
