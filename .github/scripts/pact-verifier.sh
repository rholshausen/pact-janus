#!/usr/bin/env bash
# pact-verifier.sh <dir> — installs the pact_verifier_cli release the differential harness is pinned to
# (staged plan S1.1, tools/verifier-diff/pact-verifier.json) into <dir>, and prints its path.
#
# The asset is the one pinned for this platform, and its SHA-256 must be the pinned one: a download
# with any other digest is refused, so the ledger is always judged against the verifier it was kept
# against. Moving the pin is a change to that file, not to this script.
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: pact-verifier.sh <dir>" >&2
  exit 2
fi
dir="$1"
pin="$(git rev-parse --show-toplevel)/tools/verifier-diff/pact-verifier.json"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) triple=x86_64-unknown-linux-gnu ;;
  *) echo "pact-verifier.sh: no pinned asset for $(uname -s)-$(uname -m) in $pin" >&2; exit 2 ;;
esac

release="$(jq -r '.release' "$pin")"
file="$(jq -r --arg t "$triple" '.assets[$t].file' "$pin")"
sha256="$(jq -r --arg t "$triple" '.assets[$t].sha256' "$pin")"
if [ "$file" = null ] || [ "$sha256" = null ]; then
  echo "pact-verifier.sh: $pin pins nothing for $triple" >&2
  exit 2
fi

mkdir -p "$dir"
curl -fsSL --retry 3 -o "$dir/$file" "$release/$file"
echo "$sha256  $dir/$file" | sha256sum --check --quiet - >&2
gunzip -f "$dir/$file"
binary="$dir/${file%.gz}"
chmod +x "$binary"
"$binary" --version >&2
echo "$binary"
