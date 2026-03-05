# Airplane Coder

Rust coding agent. Single binary, local models via Ollama.

## Structure

src/lib.rs — library crate. src/main.rs — CLI entry (clap: --self-test, --repl).
src/types.rs — shared types (Message, ToolCall, ChatRequest/Response).
src/ollama.rs — HTTP client for Ollama. src/anthropic.rs — HTTP client for Anthropic API.
src/agent.rs — agent loop + LlmBackend dispatch. Max 20 iterations per turn.
src/tools/mod.rs — tool dispatch. Tools: read_file, write_file, edit_file, shell, grep, glob.
src/tools/{read,write,edit,shell,grep,glob}.rs — each exports definition() + execute().
src/tui/mod.rs — TUI event loop. src/tui/widgets.rs — ratatui rendering.
tests/integration.rs — serialization, tool validity, error handling tests.

## Rules

- One job per file. Don't mix concerns.
- Shared types go in types.rs. Tools are self-contained in their own files.
- Tools return Result<String>. Errors become "Error: ..." strings (model can recover).
- Use anyhow + .context() for errors. Never unwrap() on user data. Never panic in tools.
- No unnecessary abstractions. Enum dispatch, not trait objects.
- Keep changes minimal. Don't refactor code you weren't asked to change.

## Verification tools

These are available via the shell tool. Use them when appropriate, not after every edit.

- `cargo check` — fast compile check (~30s). Good for verifying edits compile.
- `cargo test` — full build + run all tests (~2-3 min). Use when the user asks or after significant changes.
- `cargo run -- --self-test` — end-to-end smoke test (needs Ollama running).
- `git diff` — review what changed. `git status` — see working tree state.

## Adding a tool

1. Create src/tools/yourtool.rs with definition() and execute()
2. Wire into src/tools/mod.rs: add to get_tool_definitions() and execute_tool()
3. Add unit tests in the tool file
