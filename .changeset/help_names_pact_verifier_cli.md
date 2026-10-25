---
default: patch
---

# `janus --help` names pact_verifier_cli, and groups the global options

The last part of staged plan S1.29's review of the CLI's output, all in `--help`.

- Where an option behaves as today's verifier's does, the help names it: `pact_verifier_cli`.
- `--log-level` and `--no-colour` are listed under "Global options" in every command's help, not
  among the command's own options.
- Every option read from the environment is shown as `[env: NAME]`. Its value is never shown: help
  printed whatever the environment held, credentials aside, so its output depended on the shell.
