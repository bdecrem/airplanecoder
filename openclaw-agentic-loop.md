# OpenClaw Agentic Loop — Deep Dive

How OpenClaw structures its agent loop to let users accomplish coding tasks effectively.

## Overview

OpenClaw runs a **single embedded agent runtime** derived from **pi-mono**. The agent operates within a **workspace directory** that serves as the working directory for all tools, context, and persistent state. Everything — memory, config, identity — lives as plain Markdown files in this workspace.

## Session Bootstrap

On each session start, the agent's context is seeded by injecting files from the workspace:

| File | Purpose |
|------|---------|
| `AGENTS.md` | Operating instructions and directives ("memory" of how to behave) |
| `SOUL.md` | Persona, tone, and behavioral boundaries |
| `TOOLS.md` | User-maintained notes on tool conventions and coding preferences |
| `IDENTITY.md` | Agent name, vibe, emoji |
| `USER.md` | User profile and preferred way to be addressed |
| `BOOTSTRAP.md` | One-time first-run ritual (auto-deleted after completion) |

### Context Management Rules

- Large files are **trimmed and truncated** with markers to keep prompts lean
- Missing files are silently skipped (no errors — graceful degradation)
- Blank files inject a single marker line
- Missing files trigger OpenClaw to create safe default templates
- Bootstrap creation can be disabled for pre-seeded workspaces via `{ agent: { skipBootstrap: true } }`

This means the user controls the agent's personality, tools knowledge, and working style just by editing Markdown files.

## The Agentic Loop

The core loop follows this sequence:

```
User message
    |
    v
[1] Prompt Assembly
    - System prompt built from bootstrap files (AGENTS.md, SOUL.md, etc.)
    - User message combined with memory context
    - Complete prompt sent to model
    |
    v
[2] Model Invocation
    - Model processes prompt, generates response
    - Response may contain text, tool calls, or both
    |
    v
[3] Tool Execution
    - Each tool call is executed in the workspace context
    - Tool results collected
    |
    v
[4] Feed Results Back
    - Tool outputs appended to conversation
    - Loop back to step [2]
    |
    v
[5] Termination
    - When the model stops calling tools (reaches a conclusion)
    - Final text response displayed to user
```

This is the same basic pattern as most agentic coding loops, but OpenClaw's strength is in **how it manages context and memory** around this loop.

## Tools

Core tools are always available:

| Tool | Purpose |
|------|---------|
| `read` | Read file contents |
| `write` | Create/overwrite files |
| `edit` | Modify existing files |
| `exec` | Run shell commands |
| `apply_patch` | Efficient code patches (optional, gated by `tools.exec.applyPatch`) |
| `memory_search` | Semantic recall over indexed snippets |
| `memory_get` | Targeted read of specific Markdown file/line ranges |

Tools operate within the workspace directory. They degrade gracefully — `memory_get` returns empty instead of throwing if a file doesn't exist.

### apply_patch — Custom Patch Format

**Not unified diff.** OpenClaw uses a custom structured format that's more semantic and explicit:

```
*** Begin Patch
*** Add File: path/to/new-file.txt
+line 1
+line 2
*** Update File: src/app.ts
@@
-old line
+new line
*** Delete File: obsolete.txt
*** Move to: src/renamed.ts
*** End of File
*** End Patch
```

Key characteristics:
- **Custom markers** (`*** Begin Patch`, `*** End Patch`, `*** Add File:`, `*** Update File:`, `*** Delete File:`)
- **Section-based** rather than hunk-based — each file operation is explicit
- Supports **file renaming** via `*** Move to:` within an `*** Update File:` section
- Supports **EOF-only inserts** via `*** End of File` marker
- Paths can be relative (from workspace) or absolute

Configuration:
- **Experimental and disabled by default** — enable with `tools.exec.applyPatch.enabled`
- **Workspace-contained by default** — `tools.exec.applyPatch.workspaceOnly = true`

Why this over unified diff? The custom format is more **unambiguous for AI-generated patches**. Explicit Add/Update/Delete operations leave no room for misinterpretation, which matters when the model is authoring the patch rather than a human.

## Memory System

Memory is **plain Markdown in the workspace** — files are the single source of truth.

### Two-Layer Architecture

**Layer 1: Daily Log** — `memory/YYYY-MM-DD.md`
- Append-only daily notes
- Today + yesterday are read at session start
- Captures running context, decisions made during work

**Layer 2: Persistent Memory** — `MEMORY.md`
- Curated decisions, preferences, durable facts
- Persists across sessions
- Only loaded in main/private sessions (never in group contexts)

### Semantic Search / Indexing

`memory_search` uses **embedding-based semantic search** (not keyword/fuzzy matching).

**Provider priority** (auto-selects first available):
1. `local` — via `node-llama-cpp` if `memorySearch.local.modelPath` is configured
2. `openai` — if OpenAI key available
3. `gemini` — if Gemini key available
4. `voyage` — if Voyage key available
5. `mistral` — if Mistral key available
6. Disabled — if nothing is configured

**Local-only options** (no cloud API required):
- **node-llama-cpp** — configure `memorySearch.local.modelPath`, may need `pnpm approve-builds`
- **Ollama** — set `memorySearch.provider = "ollama"`, uses Ollama's `/api/embeddings` endpoint. Not auto-selected; must be explicitly configured.

**Vector storage**: Uses **sqlite-vec** (when available) to accelerate vector search inside SQLite. File watcher (debounced) rebuilds the index when memory files change.

This is relevant for Airplane Coder — we could use Ollama for both the LLM and the embedding model, keeping everything local.

### Memory Philosophy

> "If someone says 'remember this,' write it down (do not keep it in RAM)"

The agent never tries to "remember" things by holding them in context. Anything worth remembering gets written to disk. This makes memory inspectable, editable, and durable.

## Plugin / Hook System

The loop includes plugin hook points for extensibility:

- **Pre/post hooks** at various stages of the loop
- **Memory plugin system** — default is `memory-core`, can be disabled with `plugins.slots.memory = "none"`
- **Custom tool plugins** can be injected via extensions
- Hooks allow customization without forking or hardcoding behavior

## System Prompt Assembly

The system prompt is a **multi-layered composition** from bootstrap files:

```
[OpenClaw Base Prompt]        — Core agent instructions
[Model-specific instructions] — Tailored per model
---
[Skills/Tools Prompt]         — Available tool definitions
---
[Bootstrap Files Injected]    — From workspace:
  IDENTITY.md                 — Agent name, vibe, emoji
  SOUL.md                     — Persona, boundaries, tone
  AGENTS.md                   — Operating instructions
  TOOLS.md                    — User tool notes/conventions
  USER.md                     — User profile
---
[User Message]
```

**How injection works** (first turn of each session):
- All bootstrap files read from workspace
- Contents injected directly into the system prompt
- Large files are trimmed/truncated with markers
- Missing files get a "missing file" marker (no error)
- Blank files get a single marker line

**Context compaction**: When a session nears the context limit, OpenClaw triggers a silent agentic turn where the model is reminded to write durable memory to `MEMORY.md` before context is compacted. New sessions then load compacted context + updated memory files. This prevents losing important context across long sessions.

## Why It Works Well for Coding Tasks

1. **Workspace-centric design** — The agent operates in the same directory as the code. Tools like `read`, `edit`, `exec` work directly on the project files. No abstraction layer between the agent and the codebase.

2. **Iterative loop** — The read-analyze-edit-test-iterate cycle happens naturally. The model can run tests, see failures, fix code, and re-run — all within a single turn.

3. **TOOLS.md as coding guide** — Users write conventions and preferences in `TOOLS.md`. The agent reads this every session. "Always use ESM imports", "run `npm test` before committing", "prefer composition over inheritance" — these become durable instructions the agent follows.

4. **Memory carries context across sessions** — The agent remembers past architectural decisions, naming conventions, and what was tried before. This avoids re-explaining project context every session.

5. **Lean prompts** — Aggressive trimming means the model's context window is spent on the actual task, not bloated system prompts. More room for code = better code output.

6. **Transparent state** — All agent state is in readable Markdown files. Users can inspect what the agent "knows", correct mistakes, and tune behavior. Nothing is hidden in a database or opaque config.

## Key Design Principles

| Principle | How It's Implemented |
|-----------|---------------------|
| Workspace-centric | All operations scoped to a single directory |
| File-based state | Memory and config are plain Markdown (human-readable, editable) |
| Graceful degradation | Tools return empty/null instead of errors for missing files |
| Extensible via plugins | Hook points at each loop stage, pluggable memory system |
| Lean prompts | Context trimmed and truncated to keep token usage efficient |
| Disk over RAM | Persistent memory written to files, not held in context |

## Comparison to Airplane Coder

| Aspect | OpenClaw | Airplane Coder |
|--------|----------|---------------|
| Model | Cloud LLMs | Local Qwen via Ollama |
| Memory | Two-layer Markdown system | None (stateless) |
| Bootstrap | 6 injectable Markdown files | Single system prompt |
| Plugins | Hook-based extensibility | None |
| Tools | Core + pluggable | Fixed set (read, write, edit, shell, grep, glob) |
| Loop | Same pattern | Same pattern |
| Context mgmt | Aggressive trimming + memory search | Full conversation history |

The core loop structure is essentially the same. Where OpenClaw differentiates is in **context management** (bootstrap files, memory layers, trimming) and **extensibility** (plugins, hooks). These are areas Airplane Coder could adopt to improve multi-session workflows and user customization.
