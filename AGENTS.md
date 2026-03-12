# AGENTS.md

## Project: dude

A Rust CLI shell companion that intercepts command-not-found errors, suggests corrections via local learning + LLM (ollama or Claude), and supports direct queries, pipe mode, and conversational context.

## Architecture

### Binary: `dude`

Single binary built with `cargo build --release`. Optimized for size (`opt-level = "z"`, LTO, stripped).

### Core Flow

1. **command-not-found** → shell plugin calls `dude cnf <cmd> [args]`
2. **Fast path**: SQLite corrections DB checked first (instant, no network)
3. **Slow path**: LLM query via ollama or Claude API
4. **Learning**: accepted corrections stored in SQLite, become instant after 3 accepts

### Modules

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI entry (clap). Bare args treated as queries: `dude "question"` |
| `suggest.rs` | Core logic: local DB fast path → LLM slow path. Provider dispatch. |
| `ollama.rs` | Ollama `/api/generate` API. Handles reasoning models (qwen3 thinking field extraction). |
| `claude.rs` | Claude `/v1/messages` API. OAuth from macOS Keychain or API key. |
| `context.rs` | Prompt building: system prompt, user context, session history, pipe content, last command. |
| `session.rs` | Conversation memory in JSONL. 15-min TTL, max 10 entries. |
| `corrections.rs` | SQLite DB for typo→correction mappings with confidence threshold (count >= 3). |
| `safety.rs` | Destructive command detection, safe command whitelist, safety mode logic. |
| `filter.rs` | Secret redaction (KEY, TOKEN, SECRET, PASSWORD patterns) before LLM calls. |
| `profile.rs` | User profiling from shell history (top commands, tools, directories). |
| `history.rs` | Shell history reader. Supports zsh extended format and bash format. |
| `config.rs` | TOML config with `effective_*()` methods for fallback defaults. |
| `audit.rs` | JSONL audit log of all interactions. |
| `tui.rs` | Ratatui interactive config editor with arrow key navigation and popups. |

### Shell Plugins

| File | Shell | Hook |
|------|-------|------|
| `plugin/dude.plugin.zsh` | zsh | `command_not_found_handler` + `accept-line` zle widget for `?` prefix |
| `plugin/dude.bash` | bash | `command_not_found_handle` + `PROMPT_COMMAND` for last-cmd tracking |
| `plugin/dude.fish` | fish | `fish_command_not_found` + `fish_postexec` event |

### Config Location

- macOS: `~/Library/Application Support/dude/`
- Linux: `~/.config/dude/`

Files: `config.toml`, `profile.toml`, `corrections.db`, `history.jsonl`, `session.jsonl`

### Provider Auth

- **Ollama**: No auth needed, just `ollama_url` in config
- **Claude API key**: `claude_api_key` in config or `ANTHROPIC_API_KEY` env
- **Claude OAuth**: Auto-reads from macOS Keychain (`Claude Code-credentials`), uses `Authorization: Bearer` + `anthropic-beta: oauth-2025-04-20`

## Building

```bash
cargo build --release
```

Binary at `target/release/dude`. Dependencies: `clap`, `reqwest` (blocking), `rusqlite` (bundled), `serde`, `chrono`, `colored`, `ratatui`, `crossterm`.

## Testing

```bash
cargo test
```

The `filter.rs` module has unit tests for secret redaction.

## Key Design Decisions

- **Blocking HTTP**: Uses `reqwest::blocking` instead of async. Simpler, faster startup, fine for a CLI that makes 1 request per invocation.
- **SQLite bundled**: `rusqlite` with `bundled` feature compiles SQLite from source. No system dependency.
- **Reasoning model support**: Ollama's `qwen3` puts answers in a `thinking` field with empty `response`. We extract the answer from thinking content and give reasoning models a higher token budget (1000 vs 300).
- **zle widget for ?**: `preexec` can't prevent command execution in zsh. We override `accept-line` to intercept `?` before zsh parses it as a glob.
- **No hardcoded defaults**: Config fields are empty strings by default. `effective_*()` methods provide fallbacks. First run prompts setup via `dude config`.
