#!/usr/bin/env bash
set -euo pipefail

STRICT=true

for arg in "$@"; do
  case "$arg" in
    --no-strict) STRICT=false ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

if [ "$STRICT" = true ]; then
  ./target/release/sdd-coverage scan \
    --requirements requirements.yaml \
    --tasks tasks.yaml \
    --source . \
    --strict
else
  ./target/release/sdd-coverage scan \
    --requirements requirements.yaml \
    --tasks tasks.yaml \
    --source .
fi
