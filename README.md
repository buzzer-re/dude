# The Dude (⌐■_■)

**The shell companion for REALLY lazy people**

https://github.com/user-attachments/assets/a3d2a56b-7eda-4d7d-adc9-14001a1878ff

You know when you type `gti status` instead of `git status` and your terminal just stares at you? Dude fixes that. You know when you forget the `find` flags for the 400th time? Dude knows them. You know when you're too lazy to google "how to kill process on port 3000"? Just ask Dude.

Dude sits in your shell, watches you mess up, and quietly suggests what you probably meant. It remembers your mistakes and gets faster the more you use it. All locally. No cloud unless you want it.

## What It Does

```
$ gti status
dude: git status
  run it? [Enter/n]
```

```
$ cat server.log | dude "why is this broken"
dude:
The log shows repeated connection timeouts to the database on port 5432.
Your connection pool is exhausted — 50/50 connections in use since 14:32.
```

```
$ dude "list all docker containers including stopped"
dude: docker ps -a
```

You don't even need `dude` — just type what you want in plain English:

```
$ show me large files
dude: find . -size +100M -type f
  run it? [Enter/n]
```

## How It Works

1. You type something — a typo, a question, plain English
2. Shell says "command not found" and asks Dude
3. Dude checks its local database first (instant, no network)
4. If it hasn't seen this before, it asks your LLM
5. You hit Enter (or not), Dude remembers for next time

After a few corrections, Dude stops asking the LLM entirely. It just knows.

## Providers

Dude works with two backends — pick one or both:

### Ollama (Local, Free, Private)

```bash
brew install ollama
ollama serve
ollama pull qwen2.5-coder:1.5b   # fast + small, good for corrections
```

### Claude (Smart, Fast, Not Free)

If you have [Claude Code](https://claude.ai/claude-code) installed, Dude reads your OAuth token from the macOS Keychain automatically. No config needed.

```bash
dude provider claude
```

**Recommended model:** `claude-haiku-4-5-20251001` — fast, cheap ($0.25/1M input tokens), and it can actually count to 7.

You can also use a direct API key:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
dude provider claude
```

## Install

**One-liner** (downloads pre-built binary from GitHub releases):
```bash
curl -fsSL https://raw.githubusercontent.com/buzzer-re/dude/main/install.sh | bash
```

**From source** (if you prefer or no release exists yet):
```bash
git clone https://github.com/buzzer-re/dude.git
cd dude
./install.sh
```

### Shell Support

| Shell | Plugin | How It Works |
|-------|--------|-------------|
| **Zsh** | `dude.plugin.zsh` | `command_not_found_handler` |
| **Bash** | `dude.bash` | `command_not_found_handle` (no trailing 'r') |
| **Fish** | `dude.fish` | `fish_command_not_found` event, auto-loads from `conf.d/` |

## Usage

```bash
# Just type wrong, Dude catches it
gti stauts                      # → git status

# Ask anything — no subcommand needed
dude "kill process on port 3000"

# Pipe stuff in for analysis
cat crash.log | dude "summarize"
kubectl get pods | dude "which ones are failing"

# Just type in natural language
find large files
now only in the home directory

# Manage Dude
dude config                     # Interactive TUI settings
dude status                     # Check provider + model
dude model qwen2.5-coder:7b    # Swap model
dude provider claude            # Switch to Claude
dude context "test query"       # See exactly what gets sent to the LLM
dude learn                      # Re-analyze shell history
dude profile                    # See what Dude knows about you
dude history                    # Past interactions
dude clear                      # Wipe conversation session
dude forget                     # Nuclear option — wipe all learned data
```

## Config

Run `dude config` for an interactive TUI, or edit `~/Library/Application Support/dude/config.toml`:

```toml
provider = "claude"                          # or "ollama"
model = "qwen2.5-coder:1.5b"                # ollama model
claude_model = "claude-haiku-4-5-20251001"   # claude model
safety_mode = "auto"                         # "confirm", "auto", or "yolo"
ollama_url = "http://localhost:11434"
history_context = 20
```

### Safety Modes

| Mode | Behavior |
|------|----------|
| `confirm` | Always asks before running (default) |
| `auto` | Safe commands (`ls`, `git status`, etc.) run immediately, others ask |
| `yolo` | Never asks. Live dangerously. |

Destructive commands (`rm -rf /`, `dd if=/dev/zero`, etc.) are **always blocked** regardless of mode.

## How Dude Learns

- **Shell History** — On first run, Dude analyzes your history to learn what tools you use, your common directories, and your command style
- **Corrections Database** — Every time you accept a suggestion, Dude records it in a local SQLite database. After 3 accepted corrections for the same typo, it becomes instant (no LLM needed)
- **Session Memory** — Dude remembers the last few exchanges for 15 minutes, so follow-up questions work naturally
- **Secret Filtering** — Environment variables containing KEY, TOKEN, SECRET, PASSWORD are automatically redacted before anything is sent to the LLM

## Transparency

```bash
dude context "find large files"
```

Shows you **exactly** what would be sent to the LLM — system prompt, user context, history, session. No hidden data.

## Uninstall

```bash
./uninstall.sh
```
