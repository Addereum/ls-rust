#!/usr/bin/env bash

set -e

INSTALL_PATH="/usr/local/bin/ls"
BACKUP_PATH="/usr/local/bin/ls.backup"

# ---- Root Check ----
if [ "$EUID" -ne 0 ]; then
    echo "This uninstaller must be run with sudo."
    echo "Example:"
    echo "  sudo ./uninstall.sh"
    exit 1
fi

echo "==> Starting uninstall of ls-rust"

# ---- Check installation ----
if [ ! -f "${INSTALL_PATH}" ]; then
    echo "No ls found at ${INSTALL_PATH}."
    echo "Nothing to uninstall."
    exit 0
fi

# ---- Restore backup if exists ----
if [ -f "${BACKUP_PATH}" ]; then
    echo "Restoring original ls from backup."
    mv "${BACKUP_PATH}" "${INSTALL_PATH}"
    chmod +x "${INSTALL_PATH}"
    echo "Original ls restored."
else
    echo "No backup found. Removing installed ls."
    rm -f "${INSTALL_PATH}"
    echo "Removed ${INSTALL_PATH}."
fi

echo "==> Uninstall complete."
echo
echo "You may need to run:"
echo "  hash -r"
echo "to refresh your shell command cache."