#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_DIR="$SCRIPT_DIR/.venv"

# Bootstrap venv with uv if needed
if [ ! -d "$VENV_DIR" ]; then
    echo "Setting up spreadsheets venv..." >&2
    uv venv "$VENV_DIR" >&2
    uv pip install --python "$VENV_DIR/bin/python" openpyxl >&2
fi

exec "$VENV_DIR/bin/python" "$SCRIPT_DIR/spreadsheets.py" "$@"
