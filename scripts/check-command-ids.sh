#!/usr/bin/env bash
# Checks that every engine command the tools emit is registered in the SolidCraft engine.
# Usage: scripts/check-command-ids.sh [path-to-solidcraft]   (default: ./solidcraft, the submodule)
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
engine="${1:-$root/solidcraft}/crates/engine/src"
list="$root/crates/tools/src/command.rs"

[ -d "$engine" ] || { echo "no SolidCraft engine at $engine (run: git submodule update --init)" >&2; exit 2; }

ids="$(sed -n '/BEGIN ENGINE COMMANDS/,/END ENGINE COMMANDS/p' "$list" | grep -o '"[a-z_.]*"' | tr -d '"')"
[ -n "$ids" ] || { echo "no command ids found in $list" >&2; exit 2; }

missing=0
for id in $ids; do
  if grep -rqF "CommandSpec::new(\"$id\"" "$engine"; then
    echo "ok       $id"
  else
    echo "MISSING  $id" >&2
    missing=1
  fi
done
exit "$missing"
