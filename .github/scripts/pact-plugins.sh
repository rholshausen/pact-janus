#!/usr/bin/env bash
# pact-plugins.sh <dir> — installs the pact-plugins releases the plugin bridge's conformance cases are
# run against (staged plan S5.2c, plugin-bridge/pact-plugins.json) into <dir>, laid out as a plugin
# directory (<dir>/<name>-<version>/), for PACT_PLUGIN_DIR to name.
#
# Each manifest and executable must have its pinned SHA-256: a download with any other digest is
# refused, and no test installs a plugin from the plugin index (ADR 0106). An executable is written as
# pact-plugin-driver's own installer writes it, the manifest's entryPoint, with .exe on Windows. A plugin
# already installed with the pinned manifest is left alone, so a cached directory is reused.
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: pact-plugins.sh <dir>" >&2
  exit 2
fi
dir="$1"
pin="$(git rev-parse --show-toplevel)/plugin-bridge/pact-plugins.json"

exe=""
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) platform=linux-x86_64 ;;
  Linux-aarch64 | Linux-arm64) platform=linux-aarch64 ;;
  Darwin-x86_64) platform=macos-x86_64 ;;
  Darwin-arm64) platform=macos-aarch64 ;;
  MINGW*-x86_64 | MSYS*-x86_64 | CYGWIN*-x86_64) platform=windows-x86_64; exe=".exe" ;;
  *) echo "pact-plugins.sh: no pinned plugins for $(uname -s)-$(uname -m) in $pin" >&2; exit 2 ;;
esac

# Read from stdin, so no file name is printed: GNU sha256sum escapes a name holding a backslash and then
# puts a '\' before the digest, which a Windows path written with backslashes (D:\a\_temp) always does.
if command -v sha256sum >/dev/null; then
  digest() { sha256sum < "$1" | cut -d' ' -f1; }
else
  digest() { shasum -a 256 < "$1" | cut -d' ' -f1; }
fi

# check <file> <sha256>: the file has the pinned digest, or it is deleted and the script fails.
check() {
  local actual
  actual="$(digest "$1")"
  if [ "$actual" != "$2" ]; then
    rm -f "$1"
    echo "pact-plugins.sh: $1 has SHA-256 $actual, and $pin pins $2" >&2
    exit 1
  fi
}

mkdir -p "$dir"
count="$(jq '.plugins | length' "$pin")"
for i in $(seq 0 $((count - 1))); do
  name="$(jq -r ".plugins[$i].name" "$pin")"
  version="$(jq -r ".plugins[$i].version" "$pin")"
  release="$(jq -r ".plugins[$i].release" "$pin")"
  manifest_file="$(jq -r ".plugins[$i].manifest.file" "$pin")"
  manifest_sha="$(jq -r ".plugins[$i].manifest.sha256" "$pin")"
  file="$(jq -r --arg p "$platform" ".plugins[$i].executables[\$p].file" "$pin")"
  sha="$(jq -r --arg p "$platform" ".plugins[$i].executables[\$p].sha256" "$pin")"
  if [ "$file" = null ] || [ "$sha" = null ]; then
    echo "pact-plugins.sh: $pin pins no $name executable for $platform" >&2
    exit 2
  fi
  target="$dir/$name-$version"
  if [ -f "$target/pact-plugin.json" ] && [ "$(digest "$target/pact-plugin.json")" = "$manifest_sha" ] \
    && [ -f "$target/.installed" ] && [ "$(cat "$target/.installed")" = "$sha" ]; then
    echo "pact-plugins.sh: $name $version is installed" >&2
    continue
  fi
  rm -rf "$target"
  mkdir -p "$target"
  curl -fsSL --retry 3 -o "$target/pact-plugin.json" "$release/$manifest_file"
  check "$target/pact-plugin.json" "$manifest_sha"
  curl -fsSL --retry 3 -o "$target/$file" "$release/$file"
  check "$target/$file" "$sha"
  entry="$(jq -r '.entryPoint' "$target/pact-plugin.json")"
  gunzip -c "$target/$file" > "$target/$entry$exe"
  rm "$target/$file"
  chmod +x "$target/$entry$exe"
  echo "$sha" > "$target/.installed"
  echo "pact-plugins.sh: installed $name $version" >&2
done
echo "$dir"
