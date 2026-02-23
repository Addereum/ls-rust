#!/usr/bin/env bash
set -euo pipefail

TARGET_TRIPLE="x86_64-unknown-linux-musl"
BINARY_PATH="target/${TARGET_TRIPLE}/release/ls"
INSTALL_PATH="/usr/local/bin/ls"
BACKUP_PATH="/usr/local/bin/ls.backup"

# --- Determine invoking user (the one who ran sudo) ---
INVOKING_USER="${SUDO_USER:-$USER}"

# --- Helper: run a command as the invoking user (even if script is run via sudo) ---
run_as_user() {
  if [[ "${EUID}" -eq 0 && -n "${SUDO_USER:-}" ]]; then
    sudo -u "${INVOKING_USER}" -H "$@"
  else
    "$@"
  fi
}

echo "==> Starting installation of ls-rust"
echo "==> Using build user: ${INVOKING_USER}"

# ---- Check rust (cargo) for invoking user ----
if ! run_as_user bash -lc 'command -v cargo >/dev/null 2>&1'; then
  echo "Rust (cargo) not found for user '${INVOKING_USER}'."
  echo "Install Rust first:"
  echo "  curl https://sh.rustup.rs -sSf | sh"
  exit 1
fi

# ---- Ensure musl target (using invoking user's rustup) ----
if ! run_as_user bash -lc "rustup target list | grep -q '^${TARGET_TRIPLE} (installed)$'"; then
  echo "Installing Rust target ${TARGET_TRIPLE}"
  run_as_user bash -lc "rustup target add ${TARGET_TRIPLE}"
fi

# ---- Check musl-tools (system-wide) ----
if ! command -v musl-gcc >/dev/null 2>&1; then
  echo "musl-tools (musl-gcc) not found."
  echo "On Ubuntu run:"
  echo "  sudo apt install musl-tools"
  exit 1
fi

echo "==> Building release binary"
run_as_user bash -lc "cargo build --release --target ${TARGET_TRIPLE}"

if [[ ! -f "${BINARY_PATH}" ]]; then
  echo "Build failed: binary not found at ${BINARY_PATH}"
  exit 1
fi

# ---- Root required only for install step ----
if [[ "${EUID}" -ne 0 ]]; then
  echo "==> Installing requires root. Re-run with:"
  echo "  sudo ./install.sh"
  exit 1
fi

# ---- Backup existing ----
if [[ -f "${INSTALL_PATH}" ]]; then
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