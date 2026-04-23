#!/usr/bin/env sh

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
uv run --script "$SCRIPT_DIR/bump-version.py" "$@"
