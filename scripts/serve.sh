#!/usr/bin/env bash
# Starts sdd-server configured to scan this repository's own codebase.
# Run `cargo build --workspace --release` first.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY="$PROJECT_ROOT/target/release/sdd-server"

if [ ! -x "$BINARY" ]; then
  echo "Binary not found: $BINARY"
  echo "Run: cargo build --workspace --release"
  exit 1
fi

SDD_PROJECT_ROOT="$PROJECT_ROOT" \
SDD_REQUIREMENTS="$PROJECT_ROOT/requirements.yaml" \
SDD_TASKS="$PROJECT_ROOT/tasks.yaml" \
SDD_SOURCE="$PROJECT_ROOT" \
SDD_PORT="${SDD_PORT:-4010}" \
RUST_LOG="${RUST_LOG:-info}" \
  exec "$BINARY"
