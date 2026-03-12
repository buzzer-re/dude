# dude

**your shell companion for when typing is hard**

you know when you type `gti status` instead of `git status` and your terminal just stares at you? dude fixes that. you know when you forget the `find` flags for the 400th time? dude knows them. you know when you're too lazy to google "how to kill process on port 3000"? just ask dude.

dude sits in your shell, watches you mess up, and quietly suggests what you probably meant. it learns your patterns, remembers your corrections, and gets smarter over time. all locally. no cloud unless you want it.

## what it does

```
$ gti status
dude: git status
  run it? [Enter/n]
```

```
$ ? find files larger than 100mb
dude: find . -size +100M -type f
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

## how it works

1. you type something wrong
2. zsh says "command not found" and asks dude
3. dude checks its local database first (instant, no network)
4. if it hasn't seen this typo before, it asks your LLM
5. you hit enter (or not), dude remembers for next time

after a few corrections, dude stops asking the LLM entirely. it just knows.

## providers

dude works with two backends — pick one or both:

### ollama (local, free, private)

```bash
brew install ollama
ollama serve
ollama pull qwen2.5-coder:1.5b   # fast + small, good for corrections
```

### claude (smart, fast, not free)

if you have [Claude Code](https://claude.ai/claude-code) installed, dude reads your OAuth token from the macOS Keychain automatically. no config needed.

```bash
dude provider claude
```

**recommended model:** `claude-haiku-4-5-20251001` — fast, cheap ($0.25/1M input tokens), and it can actually count to 7.

you can also use a direct API key:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
dude provider claude
```

## install

```bash
git clone https://github.com/yourusername/dude.git
cd dude
./install.sh
```

needs: rust toolchain (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)

the installer:
- builds the binary
- installs to `~/.local/bin/dude`
- detects your shell (zsh/bash/fish) and installs the right plugin
- runs `dude learn` to analyze your shell history

### shell support

| shell | plugin | how it works |
|-------|--------|-------------|
| **zsh** | `dude.plugin.zsh` | `command_not_found_handler` + `accept-line` widget for `?` |
| **bash** | `dude.bash` | `command_not_found_handle` (no trailing 'r') |
| **fish** | `dude.fish` | `fish_command_not_found` event, auto-loads from `conf.d/` |

## usage

```bash
# just type wrong, dude catches it
gti stauts                      # → git status

# ask anything — no subcommand needed
dude "kill process on port 3000"

# pipe stuff in for analysis
cat crash.log | dude "summarize"
kubectl get pods | dude "which ones are failing"

# ? prefix in your shell (zsh)
? how do i tar without compression

# follow-ups work (15-min session memory)
? find large files
? now only in the home directory

# manage dude
dude config                     # interactive TUI settings
dude status                     # check provider + model
dude model qwen2.5-coder:7b    # swap model
dude provider claude            # switch to claude
dude context "test query"       # see exactly what gets sent to the LLM
dude learn                      # re-analyze shell history
dude profile                    # see what dude knows about you
dude history                    # past interactions
dude clear                      # wipe conversation session
dude forget                     # nuclear option — wipe all learned data
```

## config

run `dude config` for an interactive TUI, or edit `~/Library/Application Support/dude/config.toml`:

```toml
provider = "claude"                          # or "ollama"
model = "qwen2.5-coder:1.5b"                # ollama model
claude_model = "claude-haiku-4-5-20251001"   # claude model
safety_mode = "auto"                         # "confirm", "auto", or "yolo"
ollama_url = "http://localhost:11434"
history_context = 20
```

### safety modes

| mode | behavior |
|------|----------|
| `confirm` | always asks before running (default) |
| `auto` | safe commands (`ls`, `git status`, etc.) run immediately, others ask |
| `yolo` | never asks. live dangerously. |

destructive commands (`rm -rf /`, `dd if=/dev/zero`, etc.) are **always blocked** regardless of mode.

## how dude learns

- **shell history** — on first run, dude analyzes your history to learn what tools you use, your common directories, and your command style
- **corrections database** — every time you accept a suggestion, dude records it in a local SQLite database. after 3 accepted corrections for the same typo, it becomes instant (no LLM needed)
- **session memory** — dude remembers the last few exchanges for 15 minutes, so follow-up questions work naturally
- **secret filtering** — environment variables containing KEY, TOKEN, SECRET, PASSWORD are automatically redacted before anything is sent to the LLM

## transparency

```bash
dude context "find large files"
```

shows you **exactly** what would be sent to the LLM — system prompt, user context, history, session. no hidden data.

## uninstall

```bash
./uninstall.sh
```

## architecture

```
src/
├── main.rs          cli entry point
├── suggest.rs       correction logic (local DB → LLM fallback)
├── ollama.rs        ollama API (with reasoning model support)
├── claude.rs        claude API (OAuth + API key auth)
├── context.rs       prompt building (system + user + session)
├── session.rs       conversation memory (15-min TTL)
├── corrections.rs   SQLite learning database
├── safety.rs        destructive command detection
├── filter.rs        secret redaction
├── profile.rs       user profiling from shell history
├── history.rs       shell history reader (zsh/bash)
├── config.rs        TOML config management
├── audit.rs         interaction logging
└── tui.rs           ratatui interactive config editor
```

## license

MIT
