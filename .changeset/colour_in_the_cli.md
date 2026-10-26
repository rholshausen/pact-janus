---
default: minor
---

# `janus` colours its output on a terminal

Staged plan S1.31. Until now, only the plans that `explain` and `--explain-failures` print were coloured.

- **Every command's text output is coloured on a terminal**, in one palette. A pass is green, a
  failure red, and a warning yellow. A mismatch's path is blue, and a line that explains the one
  above it is dim. That covers `verify`'s result lines and summary, the page `check` prints, and
  `upgrade`'s findings. It also covers `publish`, `component push|pull`, and the `janus <command>:`
  line that says why a command stopped. `--help` uses cargo's colours.
- **Off a terminal, nothing changes.** Through a pipe, into a file, under `NO_COLOR` or with
  `--no-colour`, the output is byte-for-byte what it was. `CLICOLOR_FORCE` asks for colour anywhere,
  as before. `--output json`, contracts and completion scripts are never coloured.
- **`--no-colour` now reaches `--help` and usage errors**, which it did not before.
