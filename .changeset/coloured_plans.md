---
default: minor
---

# `janus explain` and `janus verify --explain-failures` colour the plan on a terminal

The plan's structure is in bold, its paths and values are coloured, and every `BOOL(false)` and
`ERROR(...)` is red, so the branch that failed is easy to find. Colour only wraps the text: with it
removed, the output is the plan-grammar spec's text form exactly (spec §3.3, new).

- **Where colour appears.** On a terminal, unless `NO_COLOR` is set or `TERM` is `dumb`. Anywhere
  when `CLICOLOR_FORCE` is set, which is how a CI log that shows colour asks for it. `NO_COLOR`
  wins over both. A pipe or a file gets plain text, as before.
- **What does not change.** The engine still renders the plan, and nothing in the protocol is
  styled: the `verification/explain` result and the `verification/executed-plan` event carry the
  same text as before. `explain --plan` prints JSON, never coloured.
- **For Rust hosts.** The kernel reads a rendered plan back as tokens (`plan::tokens`), so a host
  can colour it without parsing the text form itself. Every text form the corpora record reads
  back with nothing lost.
