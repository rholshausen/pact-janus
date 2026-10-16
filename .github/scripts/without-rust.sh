#!/usr/bin/env bash
# without-rust.sh <command> [args...] — runs a command with no Rust toolchain reachable (staged plan
# S0.10: an SDK contributor needs no Rust toolchain, and CI proves it).
#
# Every directory on PATH that holds cargo, rustc or rustup is left off it, and the tools are then
# run, not looked up, to show they are gone: bash remembers where it last found a command, so a
# lookup can report one that is no longer there. Hiding works the same on every runner, where
# uninstalling does not (the macOS runners' rustup is Homebrew's, which will not uninstall itself).
#
# CI runs the SDK suites through it; run it locally the same way, from the directory a suite is in:
#   JANUS_ENGINE_RELEASE=1 ../../.github/scripts/without-rust.sh npm test
set -euo pipefail

if [ $# -eq 0 ]; then
  echo "usage: without-rust.sh <command> [args...]" >&2
  exit 2
fi

kept=()
IFS=: read -ra dirs <<<"$PATH"
for dir in "${dirs[@]}"; do
  rust=""
  for tool in cargo rustc rustup; do
    for exe in "" .exe; do
      if [ -e "$dir/$tool$exe" ]; then
        rust=yes
      fi
    done
  done
  if [ -n "$rust" ]; then
    echo "without-rust: leaving $dir off PATH" >&2
  else
    kept+=("$dir")
  fi
done
PATH="$(IFS=:; echo "${kept[*]}")"
export PATH

for tool in cargo rustc rustup; do
  if "$tool" --version >/dev/null 2>&1; then
    echo "without-rust: $tool still runs, from $(type -P "$tool")" >&2
    exit 2
  fi
done

exec "$@"
