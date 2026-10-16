#!/usr/bin/env bash
# rename.sh <name> — renames the codename to <name> across every surface it reaches (staged plan
# S0.5, ADR 0031, Documentation/codename.md), and proves the rename complete.
#
# This is the change S2.7 makes for real once the name is decided, and the change the rename
# rehearsal makes every week to a throwaway checkout, so that by then it is a command and not an
# archaeology. In order:
#
#   1. every persisted identifier's current spelling is recorded as an earlier one, in
#      engine/kernel/src/persisted/earlier.rs, so that the renamed engine still reads documents
#      recorded under it (the dual-spelling rule);
#   2. every text file's contents: `janus`, `Janus` and `JANUS` become <name>, <Name> and <NAME>;
#   3. every tracked path the codename appears in is moved, with `git mv`;
#   4. the codename must then appear nowhere — in no file and no path — but the exemptions below.
#
# Exempt: CHANGELOG.md, which is what was released under the old name; earlier.rs, which exists to
# remember it; and the repository's own name, which is S0.7's to change, not the codename's.
#
# <name> is one lowercase ASCII word: a hyphen or a second word would need a different spelling in
# each language (a crate, a Java package, a class), and is a different rename. Runs on a clean
# working tree, and leaves the result in it, staged, for review or for the suites to run against.
set -euo pipefail

old=janus
earlier=engine/kernel/src/persisted/earlier.rs
persisted=engine/kernel/src/persisted.rs
# The repository's name, wherever a URL spells it.
keep=(rholshausen/pact-janus rholshausen.github.io/pact-janus)
exempt=(":(exclude)CHANGELOG.md" ":(exclude)$earlier")

if [ $# -ne 1 ]; then
  echo "usage: rename.sh <name>" >&2
  exit 2
fi
new="$1"
if ! grep -Eq '^[a-z][a-z0-9]*$' <<<"$new" || [[ "$new" == *"$old"* ]]; then
  echo "rename.sh: '$new' is not one lowercase ASCII word without '$old' in it" >&2
  exit 2
fi
cd "$(git rev-parse --show-toplevel)"
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "rename.sh: the working tree has changes; rename a clean checkout" >&2
  exit 2
fi
if git grep -I -q -i -F -e "$new" -- .; then
  echo "rename.sh: '$new' already appears in the repository, so the rename could not be checked:" >&2
  git grep -I -n -i -F -e "$new" -- . | head -5 >&2
  exit 2
fi

Old="$(tr '[:lower:]' '[:upper:]' <<<"${old:0:1}")${old:1}"
New="$(tr '[:lower:]' '[:upper:]' <<<"${new:0:1}")${new:1}"
OLD="$(tr '[:lower:]' '[:upper:]' <<<"$old")"
NEW="$(tr '[:lower:]' '[:upper:]' <<<"$new")"
export old Old OLD new New NEW KEEP="${keep[*]}"

# Every case form of the codename, in whatever perl reads, except inside a kept string.
# shellcheck disable=SC2016 # perl's variables, not the shell's
rename_text='
  BEGIN { @keep = split / /, $ENV{KEEP}; }
  my $i = 0; my %held;
  for my $k (@keep) { while (s/\Q$k\E/\x00$i\x00/) { $held{$i++} = $k; } }
  s/$ENV{old}/$ENV{new}/g; s/$ENV{Old}/$ENV{New}/g; s/$ENV{OLD}/$ENV{NEW}/g;
  s/\x00(\d+)\x00/$held{$1}/g;
'

# 1. The dual-spelling rule: what each persisted identifier writes now, it reads from now on.
recorded=0
while IFS=$'\t' read -r name spelling; do
  NAME="$name" SPELLING="$spelling" perl -0pi -e '
    s{(pub const \Q$ENV{NAME}\E: &\[&str\] = &\[)(.*?)(\];)}
     { $1 . ($2 =~ /\S/ ? "$2, " : "") . "\"$ENV{SPELLING}\"" . $3 }se
      or die "no $ENV{NAME} in earlier.rs\n";
  ' "$earlier"
  recorded=$((recorded + 1))
done < <(perl -0ne 'while (/^pub const (\w+): Persisted =\s*Persisted::new\(\s*"([^"]+)"/mg) { print "$1\t$2\n" }' "$persisted")
if [ "$recorded" -eq 0 ]; then
  echo "rename.sh: found no persisted identifiers in $persisted" >&2
  exit 1
fi

# 2. Contents.
mapfile -t files < <(git grep -I -l -i -e "$old" -- . "${exempt[@]}")
for file in "${files[@]}"; do
  perl -pi -e "$rename_text" "$file"
done

# 3. Paths. One file at a time: a directory's name changes with the files under it.
mapfile -t paths < <(git ls-files | grep -i -e "$old" || true)
for path in "${paths[@]}"; do
  target="$(perl -pe "$rename_text" <<<"$path")"
  mkdir -p "$(dirname "$target")"
  git mv "$path" "$target"
  # The directory it left, if that emptied it. One that still holds something untracked — a build's
  # output, say — stays, and is the checkout's to clean.
  rmdir -p "$(dirname "$path")" 2>/dev/null || true
done
git add -A

# 4. Complete: no codename left, in any case, outside the exemptions.
left="$(git grep -I -n -i -e "$old" -- . "${exempt[@]}" | perl -ne '
  BEGIN { @keep = split / /, $ENV{KEEP}; }
  my $rest = $_; for my $k (@keep) { $rest =~ s/\Q$k\E//g; } print if $rest =~ /$ENV{old}/i;' || true)"
left_paths="$(git ls-files | grep -i -e "$old" || true)"
if [ -n "$left" ] || [ -n "$left_paths" ]; then
  echo "$left" | head -40
  echo "$left_paths" | head -20
  echo "rename.sh: the rename is not complete; the lines and paths above still say '$old'" >&2
  exit 1
fi

echo "renamed $old to $new: ${#files[@]} files rewritten, ${#paths[@]} paths moved, $recorded persisted identifiers now read under their earlier spelling"
