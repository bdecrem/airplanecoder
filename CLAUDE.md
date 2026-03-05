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
├── lib.rs               # Public library crate (exposes modules for tests)
├── main.rs              # CLI (clap): --self-test, --repl flags, launches TUI
├── types.rs             # Message, ToolCall, ToolDef, ChatRequest/Response
├── ollama.rs            # OllamaClient: POST /api/chat, GET /api/tags
├── anthropic.rs         # AnthropicClient: POST /v1/messages, .env key loading
├── agent.rs             # Agent loop + LlmBackend dispatch (max 20 iterations)
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
tests/
└── integration.rs       # Contract tests: serialization, tool validity, error handling
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
| `qwen3.5:0.8b` | ~1GB | Tiny, fast, testing |
| `qwen3.5:2b` | ~2.7GB | Lightweight |
| `qwen3.5:4b` | ~3.4GB | Good balance (default) |
| `qwen3.5:9b` | ~6.6GB | Best local quality |
| `gemma3:12b` | ~8GB | Google's strong coder |
| `claude-opus-4-6` | cloud | Best quality (API) |
| `claude-sonnet-4-6` | cloud | Fast + capable (API) |

## Code Practices

### Module boundaries

- One job per file. `ollama.rs` talks to Ollama. `anthropic.rs` talks to Anthropic. `agent.rs` runs the loop. Don't mix concerns.
- Shared types live in `types.rs`. If a struct is used by more than one module, it goes there.
- Tools are self-contained: each tool file owns its `definition()` and `execute()`. The only wiring point is `tools/mod.rs`.
- TUI code never touches the network. Agent code never touches the terminal. They communicate through `mpsc` channels only.

### Keep it small

- No abstractions until you need them twice. We use enum dispatch for tools and LLM backends, not trait objects.
- No config files, no plugin systems, no dependency injection. Env vars and `.env` for secrets.
- If a new dependency adds more than it saves, don't add it.

### Error handling

- Tools return `Result<String>`. Errors become `"Error: ..."` strings sent to the model — the model can recover.
- Use `anyhow` for error propagation. Use `.context()` to add useful messages at boundaries (file I/O, HTTP calls).
- Never panic in tool code. Never `unwrap()` on user-provided data.

### Testing

**Every code change must pass `cargo test` before commit.** No exceptions.

Three levels of testing:

1. **Unit tests** (in each tool file) — fast, no external deps, test the tool logic
2. **Integration tests** (`tests/integration.rs`) — test module contracts: serialization round-trips, tool definition validity, error handling, model routing
3. **Self-test** (`--self-test`) — requires running Ollama, tests real connectivity and end-to-end tool execution

When adding or changing:
- **New tool**: add unit tests in the tool file + verify `all_tool_definitions_are_valid` still passes
- **Type changes**: the serialization tests in `tests/integration.rs` catch breaking changes to the Ollama/Anthropic wire format
- **New LLM backend**: add routing test to `model_routing_dispatches_correctly`

### Serialization is the contract

The JSON format between us and Ollama/Anthropic is the most fragile part of the system. The integration tests verify:
- `ChatRequest` serializes correctly (field names, skip_serializing_if)
- `ChatResponse` deserializes with tool_calls, without tool_calls, and with string-encoded arguments
- Tool definitions round-trip through JSON
- Every tool has valid schema (type: object, properties, required)

If you change anything in `types.rs`, run `cargo test --test integration` immediately.
