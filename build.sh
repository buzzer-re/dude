#!/bin/bash
set -e

BOLD='\033[1m'
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
RED='\033[1;31m'
DIM='\033[2m'
RESET='\033[0m'

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
DIST_DIR="dist"

echo -e "${YELLOW}dude${RESET} — building v${VERSION}"
echo ""

# ─── Check dependencies ──────────────────────────────────────────────────
if ! command -v cargo &>/dev/null; then
    echo -e "${RED}error:${RESET} cargo not found. install rust: https://rustup.rs"
    exit 1
fi

# ─── Detect host target ──────────────────────────────────────────────────
HOST_TARGET=$(rustc -vV | grep host | awk '{print $2}')
echo -e "  host target: ${BOLD}$HOST_TARGET${RESET}"

# ─── Determine targets to build ──────────────────────────────────────────
ALL_TARGETS=(
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
)

if [[ "${1:-}" == "--all" ]]; then
    TARGETS=("${ALL_TARGETS[@]}")
    echo -e "  building: ${BOLD}all targets${RESET}"
elif [[ -n "${1:-}" ]]; then
    TARGETS=("$1")
    echo -e "  building: ${BOLD}$1${RESET}"
else
    TARGETS=("$HOST_TARGET")
    echo -e "  building: ${BOLD}$HOST_TARGET${RESET} (host only, use --all for cross)"
fi

echo ""

# ─── Clean dist ──────────────────────────────────────────────────────────
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# ─── Build each target ───────────────────────────────────────────────────
for target in "${TARGETS[@]}"; do
    echo -e "${DIM}building for $target...${RESET}"

    # Check if target is installed
    if ! rustup target list --installed | grep -q "$target"; then
        echo -e "  ${DIM}installing target $target...${RESET}"
        rustup target add "$target"
    fi

    # Cross-compilation env for linux aarch64
    EXTRA_ENV=""
    if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
        if command -v aarch64-linux-gnu-gcc &>/dev/null; then
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
        else
            echo -e "  ${YELLOW}warning:${RESET} aarch64-linux-gnu-gcc not found, skipping $target"
            echo -e "  ${DIM}install with: sudo apt-get install gcc-aarch64-linux-gnu${RESET}"
            continue
        fi
    fi

    cargo build --release --target "$target" --quiet 2>&1

    # Package
    ARCHIVE="dude-${target}.tar.gz"
    tar czf "$DIST_DIR/$ARCHIVE" -C "target/$target/release" dude
    SIZE=$(du -h "$DIST_DIR/$ARCHIVE" | awk '{print $1}')
    echo -e "  ${GREEN}✓${RESET} $DIST_DIR/$ARCHIVE ${DIM}($SIZE)${RESET}"
done

# ─── Generate checksums ──────────────────────────────────────────────────
echo ""
echo -e "${DIM}generating checksums...${RESET}"
cd "$DIST_DIR"
if command -v sha256sum &>/dev/null; then
    sha256sum dude-*.tar.gz > checksums.txt
elif command -v shasum &>/dev/null; then
    shasum -a 256 dude-*.tar.gz > checksums.txt
fi
cd ..
echo -e "  ${GREEN}✓${RESET} $DIST_DIR/checksums.txt"

# ─── Summary ─────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}dist ready:${RESET}"
ls -lh "$DIST_DIR"/ | tail -n +2 | while read -r line; do
    echo -e "  $line"
done

echo ""
echo -e "${DIM}to create a release:${RESET}"
echo -e "  ${BOLD}git tag v${VERSION}${RESET}"
echo -e "  ${BOLD}git push origin v${VERSION}${RESET}"
echo -e "  ${DIM}→ GitHub Actions will build + publish automatically${RESET}"
