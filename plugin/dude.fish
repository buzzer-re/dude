# dude — your shell companion (fish plugin)
#
# Install: copy to ~/.config/fish/conf.d/dude.fish
# Or source from config.fish: source ~/.config/dude/dude.fish

# Find the dude binary
if command -q dude
    set -g DUDE_BIN dude
else if test -x "$HOME/.local/bin/dude"
    set -g DUDE_BIN "$HOME/.local/bin/dude"
else
    echo "dude: binary not found. install it first." >&2
    exit 1
end

# Auto-learn on first load if no profile exists
set -l config_dir (test -n "$XDG_CONFIG_HOME" && echo "$XDG_CONFIG_HOME" || echo "$HOME/.config")
if not test -f "$config_dir/dude/profile.toml"
    $DUDE_BIN learn &>/dev/null &
    disown
end

# ─── fish_command_not_found handler ──────────────────────────────────────
function fish_command_not_found
    set -l failed_cmd $argv[1]
    set -l args $argv[2..]

    # Get suggestion from dude
    set -l suggestion ($DUDE_BIN cnf $failed_cmd $args 2>/dev/tty)
    set -l exit_code $status

    if test $exit_code -ne 0; or test -z "$suggestion"
        echo "fish: Unknown command: $failed_cmd" >&2
        return 127
    end

    # Check safety mode
    $DUDE_BIN safety-check "$suggestion" &>/dev/null
    set -l safety_exit $status

    if test $safety_exit -eq 0
        $DUDE_BIN accept "$failed_cmd" "$suggestion" &>/dev/null &
        disown
        eval $suggestion
        return $status
    end

    if test $safety_exit -eq 2
        return 127
    end

    # Needs confirmation
    read -P "  run it? [Y/n] " -n 1 response
    if test -z "$response"; or test "$response" = y; or test "$response" = Y
        $DUDE_BIN accept "$failed_cmd" "$suggestion" &>/dev/null &
        disown
        eval $suggestion
    else
        return 127
    end
end

# ─── Track last command for "why did that fail" ─────────────────────────
set -g _DUDE_LAST_CMD_FILE "/tmp/dude_last_cmd.$USER"

function _dude_postexec --on-event fish_postexec
    set -l last_exit $status
    set -l last_cmd $argv[1]

    if test -n "$last_cmd"
        printf "command: %s\nexit_code: %d\ncwd: %s\n" \
            "$last_cmd" "$last_exit" "$PWD" > "$_DUDE_LAST_CMD_FILE"

        # Smart suggestion after failed commands
        if test $last_exit -ne 0; and test $last_exit -ne 127
            echo -e "  \033[2mdude: command failed (exit $last_exit) — try: \033[0m\033[1mdude ask 'why did that fail'\033[0m" >&2
        end
    end
end

# ─── Abbreviations ───────────────────────────────────────────────────────
abbr -a dude-learn "$DUDE_BIN learn"
abbr -a dude-profile "$DUDE_BIN profile"
abbr -a dude-forget "$DUDE_BIN forget"
abbr -a dude-status "$DUDE_BIN status"
abbr -a dude-history "$DUDE_BIN history"
abbr -a dude-context "$DUDE_BIN context"
abbr -a dude-clear "$DUDE_BIN clear"
