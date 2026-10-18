---
default: major
---

# The `janus` command line is rebuilt: `--json` becomes `--output json`, and `upgrade --out` becomes `--write`

Two flags are renamed, so a script that runs `janus` must change to move to this version:

- `--json` is now `--output json` on `verify`, `check`, `upgrade`, `component push` and
  `component pull`. `--output text` is the default. This frees `--json <file>` for parity with
  `pact_verifier_cli`, where it writes a report to a file.
- `janus upgrade --out <file>` is now `--write <file>`, so it cannot be confused with `--output`.

The exit codes are unchanged: `0` done, `1` the subject failed, `2` the command could not run.
Everything else is new or fixed:

- **Stricter command lines.** A usage error now names the problem and shows the usage of the
  subcommand it is about, and exits 2. Mistakes that used to be ignored are now refused: a value
  that is not `warn` or `block`, an `--as-of` that is not a calendar date, an `--index` that is not
  a number, extra arguments, and flags that cannot be used together (`explain --spec` with `--index`
  or `--variant`; `explain --plan` with `--executed`; `upgrade --output json` with `--write` or
  `--quiet`). `component push --help` and `component pull --help` now exist.
- **`janus verify` no longer gives up on a slow run.** It used to fail with "the run stopped
  producing events" whenever 10 seconds passed with no events, such as a slow provider or a slow
  state hook. It now waits until the run ends.
- **A closed pipe is no longer a crash.** `janus verify ... | head` used to panic when `head`
  stopped reading. The command now runs to its end and exits with its own answer.
- **`janus check --output json`** now prints the "no provider shape supplied for ..." warning on
  stderr, as the text form always did.
- **New:** `--log-level <error|warn|info|debug|trace>` on every command (`RUST_LOG` still works when
  it is absent), `janus --version`, and `janus completions <shell>` for bash, zsh, fish, PowerShell
  and elvish. Logs are coloured only on a terminal, and never when `NO_COLOR` is set.
- **`janus-engine --version` and `--help`** now answer instead of waiting on stdin. SDKs start it
  with no arguments, as before. A frame whose `Content-Length` claims more bytes than the host sends
  is now an end-of-file error, not an allocation of the claimed size.
