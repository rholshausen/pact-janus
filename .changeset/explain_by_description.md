---
default: minor
---

# `janus explain --description` chooses an interaction by its description

Staged plan S1.32. Until now `explain` took the interaction only by its position, `--index`. In a large
pact, that meant counting interactions or trying one index after another.

- **`--description <TEXT>`** chooses the interaction whose description is exactly the text, or else the
  one whose description contains it, ignoring case. An exact match wins, so a description that is the
  start of another's can still be chosen.
- **Several matches, or none, is a usage error (exit 2)** that lists the candidates. Each line gives the
  position `--index` takes, the description and the provider states, which tell apart interactions a
  pact describes alike.
- It cannot be combined with `--index` or `--spec`.
