#!/usr/bin/env bash
# janus-spec.sh <out.tar.gz> — the executable specification as a release carries it (staged plan
# S0.11, ADR 0030): the corpora, the conformance suite, the behavioural specification its ids come
# from, the binding manifest and every schema set it names. Each at its path in this repository, so
# a checker reads an unpacked archive and a checkout the same way (`--spec <dir>`, `JANUS_SPEC`).
#
# Made by `git archive` from HEAD: the committed tree, nothing the working copy adds, and the same
# bytes for the same commit.
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: janus-spec.sh <out.tar.gz>" >&2
  exit 2
fi
out="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
cd "$(git rev-parse --show-toplevel)"

paths=(
  corpora
  conformance
  Documentation/specs/sdk-specification/behavioural-spec.json
  sdks/bindings.json
)
while IFS= read -r schemas; do
  paths+=("$schemas")
done < <(jq -r '.sets[].schemas' sdks/bindings.json)

git archive --format=tar.gz --output="$out" HEAD -- "${paths[@]}"
