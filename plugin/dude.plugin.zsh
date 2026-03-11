# dude — your shell companion
# https://github.com/... (TODO)
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
_dude_config_dir="${XDG_CONFIG_HOME:-$HOME/Library/Application Support}/dude"
if [[ ! -f "$_dude_config_dir/profile.toml" ]] && [[ ! -f "$HOME/.config/dude/profile.toml" ]]; then
    "$DUDE_BIN" learn &>/dev/null &!
fi

# ─── Last command tracking ──────────────────────────────────────────────
# Captures the last command and its exit code so dude can answer
# "? why did that fail"
_DUDE_LAST_CMD=""
_DUDE_LAST_CMD_FILE="/tmp/dude_last_cmd.$USER"

# ─── Spinner ────────────────────────────────────────────────────────────
_dude_spinner_pid=""

_dude_start_spinner() {
    {
        local frames=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
        local i=0
        while true; do
            printf "\r  %s ${1:-thinking}... " "${frames[$((i % ${#frames[@]} + 1))]}" >&2
            sleep 0.1
            i=$((i + 1))
        done
    } &!
    _dude_spinner_pid=$!
}

_dude_stop_spinner() {
    if [[ -n "$_dude_spinner_pid" ]]; then
        kill "$_dude_spinner_pid" 2>/dev/null
        wait "$_dude_spinner_pid" 2>/dev/null
        printf "\r\033[K" >&2
        _dude_spinner_pid=""
    fi
}

# ─── command_not_found_handler ───────────────────────────────────────────
# Called by zsh when a command is not found.
command_not_found_handler() {
    local failed_cmd="$1"
    shift
    local args=("$@")

    _dude_start_spinner "looking up"

    # Get suggestion from dude (command on stdout, user message on stderr)
    local suggestion
    suggestion=$("$DUDE_BIN" cnf "$failed_cmd" "${args[@]}" 2>/dev/tty)
    local exit_code=$?

    _dude_stop_spinner

    # Exit code 1 = no suggestion, 2 = blocked destructive command
    if [[ $exit_code -ne 0 ]]; then
        return 127
    fi

    # Empty suggestion
    if [[ -z "$suggestion" ]]; then
        return 127
    fi

    # Check safety mode — auto mode may skip confirmation
    local safety_exit
    "$DUDE_BIN" safety-check "$suggestion" &>/dev/null
    safety_exit=$?

    if [[ $safety_exit -eq 0 ]]; then
        # Safe to auto-run
        "$DUDE_BIN" accept "$failed_cmd" "$suggestion" &!
        eval "$suggestion"
        return $?
    fi

    if [[ $safety_exit -eq 2 ]]; then
        # Destructive — already blocked by dude
        return 127
    fi

    # Needs confirmation
    echo -n "  run it? [Enter/n] " >&2
    read -r -k 1 response
    echo >&2

    if [[ "$response" == $'\n' || "$response" == "y" || "$response" == "Y" || "$response" == "" ]]; then
        # Record the accepted correction so dude learns
        "$DUDE_BIN" accept "$failed_cmd" "$suggestion" &!
        # Execute the suggested command
        eval "$suggestion"
    else
        return 127
    fi
}

# ─── "?" prefix handler ─────────────────────────────────────────────────
# Allows: ? how do I find large files
_dude_preexec() {
    # Track the command for "why did that fail" context
    _DUDE_LAST_CMD="$1"

    # Check if the command starts with "?"
    if [[ "$1" == \?* ]]; then
        local question="${1#\?}"
        question="${question# }"  # trim leading space

        if [[ -z "$question" ]]; then
            return
        fi

        _dude_start_spinner

        local suggestion
        suggestion=$("$DUDE_BIN" ask "$question" 2>/dev/tty)
        local exit_code=$?

        _dude_stop_spinner

        if [[ $exit_code -ne 0 || -z "$suggestion" ]]; then
            _DUDE_HANDLED=1
            return
        fi

        echo -n "  run it? [Enter/n] " >&2
        read -r -k 1 response
        echo >&2

        if [[ "$response" == $'\n' || "$response" == "y" || "$response" == "Y" || "$response" == "" ]]; then
            eval "$suggestion"
        fi

        _DUDE_HANDLED=1
    fi
}

_dude_precmd() {
    local last_exit=$?

    # Save last command context for "why did that fail" queries
    if [[ -n "$_DUDE_LAST_CMD" && -z "$_DUDE_HANDLED" ]]; then
        printf "command: %s\nexit_code: %d\ncwd: %s\n" \
            "$_DUDE_LAST_CMD" "$last_exit" "$PWD" > "$_DUDE_LAST_CMD_FILE"

        # Smart suggestion after failed commands
        if [[ $last_exit -ne 0 && $last_exit -ne 127 ]]; then
            echo -e "  \033[2mdude: command failed (exit $last_exit) — type \033[0m\033[1m? why did that fail\033[0m\033[2m for help\033[0m" >&2
        fi
    fi

    if [[ -n "$_DUDE_HANDLED" ]]; then
        unset _DUDE_HANDLED
    fi
}

# Register hooks
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
