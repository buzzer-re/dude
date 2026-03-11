#!/bin/bash
set -e

BOLD='\033[1m'
YELLOW='\033[1;33m'
RED='\033[1;31m'
GREEN='\033[1;32m'
DIM='\033[2m'
RESET='\033[0m'

echo -e "${YELLOW}dude${RESET} — uninstalling shell companion"
echo ""

# ─── Remove binary ──────────────────────────────────────────────────────
INSTALL_DIR="$HOME/.local/bin"
if [[ -f "$INSTALL_DIR/dude" ]]; then
    rm "$INSTALL_DIR/dude"
    echo -e "  ${GREEN}✓${RESET} removed binary from ${BOLD}$INSTALL_DIR/dude${RESET}"
else
    echo -e "  ${DIM}binary not found at $INSTALL_DIR/dude — skipping${RESET}"
fi

# ─── Remove oh-my-zsh plugin ────────────────────────────────────────────
OMZ_PLUGIN_DIR="$HOME/.oh-my-zsh/custom/plugins/dude"
if [[ -d "$OMZ_PLUGIN_DIR" ]]; then
    rm -rf "$OMZ_PLUGIN_DIR"
    echo -e "  ${GREEN}✓${RESET} removed oh-my-zsh plugin from ${BOLD}$OMZ_PLUGIN_DIR${RESET}"
    echo ""
    echo -e "  ${YELLOW}note:${RESET} remove 'dude' from your plugins list in ${BOLD}~/.zshrc${RESET}"
fi

# ─── Remove standalone plugin ───────────────────────────────────────────
STANDALONE_PLUGIN="$HOME/.config/dude/dude.plugin.zsh"
if [[ -f "$STANDALONE_PLUGIN" ]]; then
    rm "$STANDALONE_PLUGIN"
    echo -e "  ${GREEN}✓${RESET} removed plugin from ${BOLD}$STANDALONE_PLUGIN${RESET}"
    echo ""
    echo -e "  ${YELLOW}note:${RESET} remove the ${BOLD}source${RESET} line for dude from ${BOLD}~/.zshrc${RESET}"
fi

# ─── Remove learned data ────────────────────────────────────────────────
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/Library/Application Support}/dude"
FALLBACK_DIR="$HOME/.config/dude"

echo ""
echo -n -e "  wipe learned data & config? [y/N] "
read -r response

if [[ "$response" == "y" || "$response" == "Y" ]]; then
    if [[ -d "$CONFIG_DIR" ]]; then
        rm -rf "$CONFIG_DIR"
        echo -e "  ${GREEN}✓${RESET} removed ${BOLD}$CONFIG_DIR${RESET}"
    fi
    if [[ -d "$FALLBACK_DIR" ]] && [[ "$FALLBACK_DIR" != "$CONFIG_DIR" ]]; then
        rm -rf "$FALLBACK_DIR"
        echo -e "  ${GREEN}✓${RESET} removed ${BOLD}$FALLBACK_DIR${RESET}"
    fi
else
    echo -e "  ${DIM}keeping learned data at $CONFIG_DIR${RESET}"
fi

echo ""
echo -e "${GREEN}dude has left the building.${RESET} restart your shell to finish cleanup."
