# dude — your shell companion
#
# This plugin intercepts command-not-found errors and routes them to dude,
# which suggests corrections using local learning + ollama/claude.

# Find the dude binary
if command -v dude &>/dev/null; then
    DUDE_BIN="dude"
elif [[ -x "${0:A:h}/bin/dude" ]]; then
    DUDE_BIN="${0:A:h}/bin/dude"
elif [[ -x "$HOME/.local/bin/dude" ]]; then
    DUDE_BIN="$HOME/.local/bin/dude"
else
    echo "dude: binary not found. install it first." >&2
    return
fi

# Auto-learn on first load if no profile exists
# Check both macOS and XDG config locations
if [[ ! -f "${XDG_CONFIG_HOME:-$HOME/.config}/dude/profile.toml" ]] && \
   [[ ! -f "$HOME/Library/Application Support/dude/profile.toml" ]]; then
    "$DUDE_BIN" learn &>/dev/null &!
fi

# ─── Last command tracking ──────────────────────────────────────────────
_DUDE_LAST_CMD=""
_DUDE_LAST_CMD_FILE="/tmp/dude_last_cmd.$USER"

# ─── command_not_found_handler ───────────────────────────────────────────
_DUDE_CNF_ACTIVE=0

command_not_found_handler() {
    # Guard against infinite loops: if dude suggests a command that also
    # doesn't exist, we'd re-enter this handler forever. Bail out if
    # we're already inside a correction.
    if [[ $_DUDE_CNF_ACTIVE -ne 0 ]]; then
        echo "zsh: command not found: $1" >&2
        return 127
    fi
    _DUDE_CNF_ACTIVE=1

    local failed_cmd="$1"
    shift
    local args=("$@")

    local suggestion
    local full_cmd="$failed_cmd${args:+ ${args[*]}}"
    suggestion=$("$DUDE_BIN" ask "$full_cmd" 2>/dev/tty)
    local exit_code=$?

    if [[ $exit_code -ne 0 || -z "$suggestion" ]]; then
        _DUDE_CNF_ACTIVE=0
        return 127
    fi

    # Check safety mode
    local safety_exit
    "$DUDE_BIN" safety-check "$suggestion" &>/dev/null
    safety_exit=$?

    if [[ $safety_exit -eq 0 ]]; then
        "$DUDE_BIN" accept "$failed_cmd" "$suggestion" &>/dev/null
        print -s "$suggestion"
        eval "$suggestion"
        local ret=$?
        _DUDE_CNF_ACTIVE=0
        return $ret
    fi

    if [[ $safety_exit -eq 2 ]]; then
        _DUDE_CNF_ACTIVE=0
        return 127
    fi

    echo -n "  run it? [Enter/n] " >&2
    read -r -k 1 response
    echo >&2

    if [[ "$response" == $'\n' || "$response" == "y" || "$response" == "Y" || "$response" == "" ]]; then
        "$DUDE_BIN" accept "$failed_cmd" "$suggestion" &>/dev/null
        print -s "$suggestion"
        eval "$suggestion"
    else
        _DUDE_CNF_ACTIVE=0
        return 127
    fi
    _DUDE_CNF_ACTIVE=0
}

# ─── Track last command for "why did that fail" ─────────────────────────
_dude_preexec() {
    _DUDE_LAST_CMD="$1"
}

_dude_precmd() {
    local last_exit=$?

    if [[ -n "$_DUDE_LAST_CMD" ]]; then
        printf "command: %s\nexit_code: %d\ncwd: %s\n" \
            "$_DUDE_LAST_CMD" "$last_exit" "$PWD" > "$_DUDE_LAST_CMD_FILE"
        _DUDE_LAST_CMD=""
    fi
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec _dude_preexec
add-zsh-hook precmd _dude_precmd

# ─── Aliases ─────────────────────────────────────────────────────────────
alias dude-learn="$DUDE_BIN learn"
alias dude-profile="$DUDE_BIN profile"
alias dude-forget="$DUDE_BIN forget"
alias dude-status="$DUDE_BIN status"
alias dude-history="$DUDE_BIN history"
alias dude-context="$DUDE_BIN context"
alias dude-clear="$DUDE_BIN clear"
