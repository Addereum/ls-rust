#!/usr/bin/env bash

set -e

TARGET_TRIPLE="x86_64-unknown-linux-musl"
BINARY_PATH="target/${TARGET_TRIPLE}/release/ls"
INSTALL_PATH="/usr/local/bin/ls"
BACKUP_PATH="/usr/local/bin/ls.backup"

# ---- Root Check ----
if [ "$EUID" -ne 0 ]; then
    echo "This installer must be run with sudo."
    echo "Example:"
    echo "  sudo ./install.sh"
    exit 1
fi

echo "==> Starting installation of ls-rust"

# ---- Check rust ----
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust (cargo) not found. Install Rust first:"
    echo "  curl https://sh.rustup.rs -sSf | sh"
    exit 1
fi

# ---- Ensure musl target ----
if ! rustup target list | grep -q "${TARGET_TRIPLE} (installed)"; then
    echo "Installing Rust target ${TARGET_TRIPLE}"
    rustup target add ${TARGET_TRIPLE}
fi

# ---- Check musl-tools ----
if ! command -v musl-gcc >/dev/null 2>&1; then
    echo "musl-tools not found."
    echo "On Ubuntu run:"
    echo "  sudo apt install musl-tools"
    exit 1
fi

echo "==> Building release binary"
cargo build --release --target ${TARGET_TRIPLE}

if [ ! -f "${BINARY_PATH}" ]; then
    echo "Build failed: binary not found."
    exit 1
fi

# ---- Backup existing ----
if [ -f "${INSTALL_PATH}" ]; then
    echo "Backing up existing ls to ${BACKUP_PATH}"
    cp "${INSTALL_PATH}" "${BACKUP_PATH}"
fi

echo "==> Installing to ${INSTALL_PATH}"
cp "${BINARY_PATH}" "${INSTALL_PATH}"
chmod +x "${INSTALL_PATH}"

echo "==> Installation complete."
echo
echo "Verify with:"
echo "  which ls"
echo
echo "Rollback (if needed):"
echo "  sudo mv ${BACKUP_PATH} ${INSTALL_PATH}"