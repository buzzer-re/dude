#!/bin/bash
set -e

BOLD='\033[1m'
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
RED='\033[1;31m'
DIM='\033[2m'
RESET='\033[0m'

# ─── Config ──────────────────────────────────────────────────────────────
GITHUB_REPO="buzzer-re/dude"
INSTALL_DIR="$HOME/.local/bin"
PLUGIN_INSTALL_DIR="$HOME/.config/dude"

echo -e "${YELLOW}dude${RESET} — installing your shell companion"
echo ""

# ─── Detect platform ─────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin) OS_TARGET="apple-darwin" ;;
    Linux)  OS_TARGET="unknown-linux-gnu" ;;
    *)
        echo -e "${RED}error:${RESET} unsupported OS: $OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64)  ARCH_TARGET="x86_64" ;;
    aarch64|arm64) ARCH_TARGET="aarch64" ;;
    *)
        echo -e "${RED}error:${RESET} unsupported architecture: $ARCH"
        exit 1
        ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"
echo -e "  platform: ${BOLD}${OS} ${ARCH}${RESET} (${TARGET})"

# ─── Try to download from GitHub releases ────────────────────────────────
download_from_release() {
    if ! command -v curl &>/dev/null; then
        return 1
    fi

    # Get latest release tag
    local latest
    latest=$(curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" 2>/dev/null \
        | grep '"tag_name"' | head -1 | sed 's/.*: "\(.*\)".*/\1/')

    if [[ -z "$latest" ]]; then
        return 1
    fi

    local url="https://github.com/${GITHUB_REPO}/releases/download/${latest}/dude-${TARGET}.tar.gz"
    echo -e "  ${DIM}found release ${latest}${RESET}"
    echo -e "  ${DIM}downloading from GitHub...${RESET}"

    local tmpdir
    tmpdir=$(mktemp -d)
    trap "rm -rf '$tmpdir'" EXIT

    if curl -fsSL "$url" -o "$tmpdir/dude.tar.gz" 2>/dev/null; then
        tar xzf "$tmpdir/dude.tar.gz" -C "$tmpdir"
        if [[ -x "$tmpdir/dude" ]]; then
            mkdir -p "$INSTALL_DIR"
            mv "$tmpdir/dude" "$INSTALL_DIR/dude"
            chmod +x "$INSTALL_DIR/dude"
            echo -e "  ${GREEN}✓${RESET} binary installed to ${BOLD}$INSTALL_DIR/dude${RESET} (${latest})"
            return 0
        fi
    fi

    return 1
}

# ─── Build from source (fallback) ────────────────────────────────────────
build_from_source() {
    local script_dir="$1"

    if ! command -v cargo &>/dev/null; then
        echo -e "${RED}error:${RESET} no pre-built binary available and cargo not found"
        echo -e "  install rust: ${BOLD}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${RESET}"
        exit 1
    fi

    echo -e "  ${DIM}building from source...${RESET}"
    cd "$script_dir"
    cargo build --release --quiet
    mkdir -p "$INSTALL_DIR"
    cp "$script_dir/target/release/dude" "$INSTALL_DIR/dude"
    chmod +x "$INSTALL_DIR/dude"
    echo -e "  ${GREEN}✓${RESET} binary installed to ${BOLD}$INSTALL_DIR/dude${RESET} (built from source)"
}

# ─── Install binary ──────────────────────────────────────────────────────
# If running from a git clone, try release first then fall back to source build.
# If running via curl|bash, only try release download.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd || echo "")"
HAS_SOURCE=false
if [[ -n "$SCRIPT_DIR" && -f "$SCRIPT_DIR/Cargo.toml" ]]; then
    HAS_SOURCE=true
fi

if download_from_release; then
    : # success
elif [[ "$HAS_SOURCE" == true ]]; then
    echo -e "  ${YELLOW}no release found, building from source...${RESET}"
    build_from_source "$SCRIPT_DIR"
else
    echo -e "${RED}error:${RESET} could not download binary from GitHub"
    echo -e "  either clone the repo and run install.sh, or check ${BOLD}https://github.com/${GITHUB_REPO}/releases${RESET}"
    exit 1
fi

# Make sure ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "  ${YELLOW}note:${RESET} add this to your shell rc: ${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
fi

# ─── Install shell plugins ───────────────────────────────────────────────
CURRENT_SHELL="$(basename "$SHELL")"
echo ""
echo -e "  detected shell: ${BOLD}$CURRENT_SHELL${RESET}"

# Download plugins if not running from source
get_plugin() {
    local plugin_name="$1"
    local dest="$2"

    if [[ "$HAS_SOURCE" == true ]]; then
        cp "$SCRIPT_DIR/plugin/$plugin_name" "$dest"
    else
        local url="https://raw.githubusercontent.com/${GITHUB_REPO}/main/plugin/${plugin_name}"
        curl -fsSL "$url" -o "$dest" 2>/dev/null || {
            echo -e "  ${RED}error:${RESET} could not download $plugin_name"
            return 1
        }
    fi
}

install_zsh() {
    if [[ -d "$HOME/.oh-my-zsh" ]]; then
        local plugin_dir="$HOME/.oh-my-zsh/custom/plugins/dude"
        mkdir -p "$plugin_dir"
        get_plugin "dude.plugin.zsh" "$plugin_dir/dude.plugin.zsh"
        echo -e "  ${GREEN}✓${RESET} plugin installed to ${BOLD}$plugin_dir${RESET}"

        if grep -q "plugins=" "$HOME/.zshrc" 2>/dev/null; then
            if ! grep -q "dude" "$HOME/.zshrc" 2>/dev/null; then
                echo ""
                echo -e "  ${YELLOW}add 'dude' to your plugins in ~/.zshrc:${RESET}"
                echo -e "  ${BOLD}plugins=(... dude)${RESET}"
            fi
        fi
    else
        mkdir -p "$PLUGIN_INSTALL_DIR"
        get_plugin "dude.plugin.zsh" "$PLUGIN_INSTALL_DIR/dude.plugin.zsh"
        echo -e "  ${GREEN}✓${RESET} plugin installed to ${BOLD}$PLUGIN_INSTALL_DIR${RESET}"

        if ! grep -q "dude.plugin.zsh" "$HOME/.zshrc" 2>/dev/null; then
            echo ""
            echo -e "  ${YELLOW}add this to your ~/.zshrc:${RESET}"
            echo -e "  ${BOLD}source \"\$HOME/.config/dude/dude.plugin.zsh\"${RESET}"
        fi
    fi
}

install_bash() {
    mkdir -p "$PLUGIN_INSTALL_DIR"
    get_plugin "dude.bash" "$PLUGIN_INSTALL_DIR/dude.bash"
    echo -e "  ${GREEN}✓${RESET} plugin installed to ${BOLD}$PLUGIN_INSTALL_DIR/dude.bash${RESET}"

    if ! grep -q "dude.bash" "$HOME/.bashrc" 2>/dev/null; then
        echo ""
        echo -e "  ${YELLOW}add this to your ~/.bashrc:${RESET}"
        echo -e "  ${BOLD}source \"\$HOME/.config/dude/dude.bash\"${RESET}"
    fi
}

install_fish() {
    local fish_dir="$HOME/.config/fish/conf.d"
    mkdir -p "$fish_dir"
    get_plugin "dude.fish" "$fish_dir/dude.fish"
    echo -e "  ${GREEN}✓${RESET} plugin installed to ${BOLD}$fish_dir/dude.fish${RESET}"
    echo -e "  ${GREEN}✓${RESET} fish auto-loads from conf.d — no config changes needed"
}

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

if [[ "${1:-}" == "--all-shells" ]]; then
    echo ""
    echo -e "${DIM}installing plugins for all shells...${RESET}"
    install_zsh
    install_bash
    install_fish
fi

# ─── Check provider ──────────────────────────────────────────────────────
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
