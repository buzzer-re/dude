#!/bin/bash
set -e

BOLD='\033[1m'
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
DIM='\033[2m'
RESET='\033[0m'

echo -e "${YELLOW}dude${RESET} — installing your shell companion"
echo ""

# ─── Build the binary ────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if ! command -v cargo &>/dev/null; then
    echo "error: cargo not found. install rust first: https://rustup.rs"
    exit 1
fi

echo -e "${DIM}building dude...${RESET}"
cd "$SCRIPT_DIR"
cargo build --release --quiet

# ─── Install binary ──────────────────────────────────────────────────────
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "$SCRIPT_DIR/target/release/dude" "$INSTALL_DIR/dude"
chmod +x "$INSTALL_DIR/dude"
echo -e "  binary installed to ${BOLD}$INSTALL_DIR/dude${RESET}"

# Make sure ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "  ${YELLOW}note:${RESET} add this to your shell rc: ${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
fi

# ─── Detect current shell ───────────────────────────────────────────────
CURRENT_SHELL="$(basename "$SHELL")"
echo ""
echo -e "  detected shell: ${BOLD}$CURRENT_SHELL${RESET}"

# ─── Install zsh plugin ─────────────────────────────────────────────────
install_zsh() {
    if [[ -d "$HOME/.oh-my-zsh" ]]; then
        PLUGIN_DIR="$HOME/.oh-my-zsh/custom/plugins/dude"
        mkdir -p "$PLUGIN_DIR"
        cp "$SCRIPT_DIR/plugin/dude.plugin.zsh" "$PLUGIN_DIR/dude.plugin.zsh"
        echo -e "  plugin installed to ${BOLD}$PLUGIN_DIR${RESET}"

        if grep -q "plugins=" "$HOME/.zshrc" 2>/dev/null; then
            if ! grep -q "dude" "$HOME/.zshrc" 2>/dev/null; then
                echo ""
                echo -e "  ${YELLOW}add 'dude' to your plugins in ~/.zshrc:${RESET}"
                echo -e "  ${BOLD}plugins=(... dude)${RESET}"
            fi
        fi
    else
        PLUGIN_DIR="$HOME/.config/dude"
        mkdir -p "$PLUGIN_DIR"
        cp "$SCRIPT_DIR/plugin/dude.plugin.zsh" "$PLUGIN_DIR/dude.plugin.zsh"
        echo -e "  plugin installed to ${BOLD}$PLUGIN_DIR${RESET}"
        echo ""
        echo -e "  ${YELLOW}add this to your ~/.zshrc:${RESET}"
        echo -e "  ${BOLD}source \"\$HOME/.config/dude/dude.plugin.zsh\"${RESET}"
    fi
}

# ─── Install bash plugin ────────────────────────────────────────────────
install_bash() {
    PLUGIN_DIR="$HOME/.config/dude"
    mkdir -p "$PLUGIN_DIR"
    cp "$SCRIPT_DIR/plugin/dude.bash" "$PLUGIN_DIR/dude.bash"
    echo -e "  plugin installed to ${BOLD}$PLUGIN_DIR/dude.bash${RESET}"

    if ! grep -q "dude.bash" "$HOME/.bashrc" 2>/dev/null; then
        echo ""
        echo -e "  ${YELLOW}add this to your ~/.bashrc:${RESET}"
        echo -e "  ${BOLD}source \"\$HOME/.config/dude/dude.bash\"${RESET}"
    fi
}

# ─── Install fish plugin ────────────────────────────────────────────────
install_fish() {
    FISH_CONF_DIR="$HOME/.config/fish/conf.d"
    mkdir -p "$FISH_CONF_DIR"
    cp "$SCRIPT_DIR/plugin/dude.fish" "$FISH_CONF_DIR/dude.fish"
    echo -e "  plugin installed to ${BOLD}$FISH_CONF_DIR/dude.fish${RESET}"
    echo -e "  ${GREEN}✓${RESET} fish auto-loads from conf.d — no config changes needed"
}

# ─── Install for detected shell + optionally others ─────────────────────
case "$CURRENT_SHELL" in
    zsh)  install_zsh ;;
    bash) install_bash ;;
    fish) install_fish ;;
    *)
        echo -e "  ${YELLOW}unknown shell '$CURRENT_SHELL' — installing all plugins${RESET}"
        install_zsh
        install_bash
        install_fish
        ;;
esac

# Install other shells if requested
if [[ "${1:-}" == "--all-shells" ]]; then
    echo ""
    echo -e "${DIM}installing plugins for all shells...${RESET}"
    install_zsh
    install_bash
    install_fish
fi

# ─── Check ollama ─────────────────────────────────────────────────────────
echo ""
if command -v ollama &>/dev/null; then
    echo -e "  ${GREEN}✓${RESET} ollama found"
    if curl -s http://localhost:11434 &>/dev/null; then
        echo -e "  ${GREEN}✓${RESET} ollama is running"
    else
        echo -e "  ${YELLOW}!${RESET} ollama installed but not running. start it: ${BOLD}ollama serve${RESET}"
    fi
else
    echo -e "  ${YELLOW}!${RESET} ollama not found. install it: ${BOLD}https://ollama.ai${RESET}"
    echo -e "  ${DIM}or set provider to claude: ${BOLD}dude provider claude${RESET}"
fi

# ─── Initial learn ────────────────────────────────────────────────────────
echo ""
echo -e "${DIM}analyzing your shell history...${RESET}"
"$INSTALL_DIR/dude" learn
echo ""
echo -e "${GREEN}dude is ready.${RESET} restart your shell or run: ${BOLD}source ~/.${CURRENT_SHELL}rc${RESET}"
