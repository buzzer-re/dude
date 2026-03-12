# dude — your shell companion (bash plugin)
#
# Source this from your ~/.bashrc:
#   source "$HOME/.config/dude/dude.bash"

# Find the dude binary
if command -v dude &>/dev/null; then
    DUDE_BIN="dude"
elif [[ -x "$HOME/.local/bin/dude" ]]; then
    DUDE_BIN="$HOME/.local/bin/dude"
else
    echo "dude: binary not found. install it first." >&2
    return
fi

# Auto-learn on first load if no profile exists
_dude_config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/dude"
if [[ ! -f "$_dude_config_dir/profile.toml" ]]; then
    "$DUDE_BIN" learn &>/dev/null &
    disown
fi

# ─── Last command tracking ──────────────────────────────────────────────
_DUDE_LAST_CMD_FILE="/tmp/dude_last_cmd.$USER"

# ─── command_not_found_handle (bash — no trailing 'r') ──────────────────
command_not_found_handle() {
    local failed_cmd="$1"
    shift
    local args=("$@")

    # Get suggestion from dude
    local suggestion
    suggestion=$("$DUDE_BIN" cnf "$failed_cmd" "${args[@]}" 2>/dev/tty)
    local exit_code=$?

    if [[ $exit_code -ne 0 || -z "$suggestion" ]]; then
        echo "bash: $failed_cmd: command not found" >&2
        return 127
    fi

    # Check safety mode
    local safety_exit
    "$DUDE_BIN" safety-check "$suggestion" &>/dev/null
    safety_exit=$?

    if [[ $safety_exit -eq 0 ]]; then
        "$DUDE_BIN" accept "$failed_cmd" "$suggestion" &>/dev/null &
        disown
        history -s "$suggestion"
        eval "$suggestion"
        return $?
    fi

    if [[ $safety_exit -eq 2 ]]; then
        return 127
    fi

    # Needs confirmation
    echo -n "  run it? [Y/n] " >&2
    read -r -n 1 response
    echo >&2

    if [[ "$response" == "" || "$response" == "y" || "$response" == "Y" ]]; then
        "$DUDE_BIN" accept "$failed_cmd" "$suggestion" &>/dev/null &
        disown
        history -s "$suggestion"
        eval "$suggestion"
    else
        return 127
    fi
}

# ─── "?" prefix — intercept via DEBUG trap + extdebug ────────────────────
# With extdebug, returning non-zero from a DEBUG trap prevents the command
# from executing. This lets us intercept "? question" before bash tries to
# run it (which would fail with "?: command not found").
shopt -s extdebug

_dude_check_question() {
    local cmd="$BASH_COMMAND"

    # Only intercept "? ..." at the top level
    if [[ "$BASH_SUBSHELL" -eq 0 && "$cmd" == "?"* && -z "$_DUDE_IN_QUESTION" ]]; then
        local question="${cmd#\?}"
        question="${question# }"

        if [[ -n "$question" ]]; then
            _DUDE_IN_QUESTION=1

            local suggestion
            suggestion=$("$DUDE_BIN" ask "$question" 2>/dev/tty)
            local exit_code=$?

            if [[ $exit_code -eq 0 && -n "$suggestion" ]]; then
                echo -n "  run it? [Y/n] " >&2
                read -r -n 1 response
                echo >&2

                if [[ "$response" == "" || "$response" == "y" || "$response" == "Y" ]]; then
                    history -s "$suggestion"
                    eval "$suggestion"
                fi
            fi

            unset _DUDE_IN_QUESTION
            return 1  # non-zero = don't execute the original command
        fi
    fi
    return 0  # let everything else through
}
trap '_dude_check_question' DEBUG

# ─── PROMPT_COMMAND for tracking last command ────────────────────────────
_dude_prompt_command() {
    local last_exit=$?

    # Get last command from history
    local last_cmd
    last_cmd=$(HISTTIMEFORMAT= history 1 | sed 's/^[ ]*[0-9]*[ ]*//')

    if [[ -n "$last_cmd" ]]; then
        printf "command: %s\nexit_code: %d\ncwd: %s\n" \
            "$last_cmd" "$last_exit" "$PWD" > "$_DUDE_LAST_CMD_FILE"

    fi
}

# Append to PROMPT_COMMAND rather than overwriting
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="_dude_prompt_command"
else
    PROMPT_COMMAND="_dude_prompt_command;$PROMPT_COMMAND"
fi

# ─── Aliases ─────────────────────────────────────────────────────────────
alias dude-learn="$DUDE_BIN learn"
alias dude-profile="$DUDE_BIN profile"
alias dude-forget="$DUDE_BIN forget"
alias dude-status="$DUDE_BIN status"
alias dude-history="$DUDE_BIN history"
alias dude-context="$DUDE_BIN context"
alias dude-clear="$DUDE_BIN clear"
