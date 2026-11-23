# 0108 — A subprocess component is a program and its arguments, and its process is the unit of recreation

- **Status**: accepted
- **Date**: 2026-11-23
- **Plan tasks**: S5.2b (the subprocess loader, moved from S5.5 by ADR 0106); S5.5, which keeps the author
  guide's section and the general-availability statement
- **Evidence**: spike 8.3's [findings](../../spikes/8.3-subprocess-transport/FINDINGS.md) and its loader
  (`b8d5010`, `spikes/8.3-subprocess-transport/src/lib.rs`); [phase-9 findings](../prototype/phase-9-findings.md)
  18 and 20; component-interfaces spec §3.4 and §9.3; lifecycle-hooks spec §7.1 and §8.3 (an `exec`
  hook's `command`, `args` and `cwd`); `engine/component-host/tests/subprocess.rs` and
  `third-party/janus-csv-js`, which this item built

## Context

Spec §9.3 specifies the subprocess binding: a component the engine spawns, speaking component frames
over standard input and output with `Content-Length` framing, exiting when its standard input closes.
Spike 8.3 proved it with a Node transport and deleted its loader with the rest of the spike code. ADR
0106 makes it the binding `janus-plugin-bridge` is reached through, so S5.2b builds the loader for real.
Two findings from the spike stood in the way.

**Finding 20: what a source's command is.** The schema says a `subprocess` source's `reference` is "the
command the engine spawns" and no more: not whether it is a shell string or a program, nor what a
relative path in it is relative to. The spike split it on whitespace, which breaks on the first path
with a space in it, and `node components/csv.mjs` meant one thing from the project's root and another
from anywhere else.

**Finding 18: what "recreate the instance" means out of process.** Spec §3.4 says an instance that timed
out is never reused and is recreated. Spec §9.3 lets one process host several instances. Out of process
the only way to stop a call is to stop the process, so one hung `transport/poll-inbound` on one
session's mock closes every other session's listening socket. The spike showed it, and the process that
replaced the dead one had never heard of the instances its predecessor held.

## Decision

1. **A `subprocess` source is a program, its arguments and a directory, never a shell string.**
   `reference` is the program. `args`, an array of strings beside it, are its arguments, passed as they
   are written, through no shell. `cwd` is the directory it starts in. This is an `exec` hook's
   `command`, `args` and `cwd` (lifecycle-hooks spec §8.3), so a project spells a process one way.
2. **Paths mean the same from wherever a run starts.** The configuration loader resolves a relative
   `cwd` against the configuration file's directory, and gives a `subprocess` source with no `cwd` that
   directory, so `args: [components/csv.mjs]` names the same file from anywhere. A `reference` with no
   path separator in it is a program found on the engine's `PATH` (with the platform's executable
   extensions on Windows), because the component's own environment is empty. A `reference` with one is
   a path, resolved against `cwd`. A declaration that reaches the engine with no configuration file, as
   an SDK's `components` option may, is read as written, relative to the engine's own directory.
3. **Out of process, the unit of recreation is the process.** A call whose deadline passes ends the
   process, as `component-timeout`, and one whose process ends is `component-exited` at once, not at its
   deadline, since standard output closing is the signal. Either way every instance the process held
   goes with it. That is spec §3.4's rule as it has to be read out of process, and spec §3.4 now says so.
   Content and matcher calls name no instance and are stateless, so the next one is made on a new process,
   handshaken before it is asked anything else. A new process that introduces itself as another component,
   or another version, fails that call as `component-malformed` and is ended.
4. **The loader says which instances went with a process.** A transport instance lives in the process
   that answered its `transport/start`, until its `transport/stop`. The error that ends a process names
   the instances it held, as `details.instances`, and a later call on any of them fails at once as
   `component-exited`, with `details.lost: true`, naming the process it went with, rather than reaching a
   new process that never started it. The exchange loop serving one logs that error as it stops. A
   component author still chooses how many instances share a process: one with a costly start-up keeps
   one process, one that would rather lose less starts a transport per process of its own (S5.5's
   author guide section says how).
5. **A call's deadline is the declaration's, plus the time the call is meant to wait.**
   `limits.deadline-ms` (10 seconds by default) bounds every call, and a `transport/poll-inbound` or
   `transport/send` that carries a `timeout-ms` has it added, so a poll that waits 200 ms is not a slow
   one. A frame that cannot be attributed to a call (not JSON, or an id nobody asked with) is skipped and
   counted, and when exactly one call is waiting it fails that call at once as `component-malformed`,
   since it can have come from nothing else (spike 8.3 finding 3).
6. **The process's environment is empty, plus exactly the `env` grant**, and `SystemRoot` on Windows
   (spec §9.3, phase-9 finding 19). Its standard error is the engine's log, line by line, at `info`, under
   the component's name. Text on standard output that is not a frame is skipped with a warning and the
   stream stays in sync. `fs` and `network` grants are not enforced, and a load that declares either
   warns once that they are not.
7. **Every interface the pipe defines is bound, but `hook`.** Content and matcher calls go through the
   kernel's `component::wire::PipeClient`, as the WASM binding's do. Transport calls go through
   `wire::TransportClient`, written beside it, with the kind's declarations (`content-slots`,
   `exchange-parts`, `credential-members`, `outcome-slots`) read from the handshake. A component that
   implements `hook` still fails to load as `component-invalid` (ADR 0104; hooks are S5.9).
8. **`janus` and `janus-engine` register the loader,** so `engine/hello` lists `subprocess` among
   `components.loaders`. A subprocess component is recorded in a contract and a summary by its handshake's
   name and version and its source's kind alone (ADR 0097): it has no digest, and a path is not recorded.
9. **`janus component check` takes a declared component as well as a `.wasm`.** With `--config`, its
   first argument names a component the configuration declares, of any source kind, loaded as a run
   loads it, so a subprocess component's cases run through the subprocess binding. Without it, the first
   argument is a `.wasm` file, as before.

## Alternatives considered

- **`reference` as a shell string run through the platform shell** (finding 20's option B). It reads
  naturally, but `sh -c` and `cmd /c` quote differently, so a declaration would be portable only in name,
  and a shell adds a process between the engine and the component that a kill has to reach through.
- **A `reference` that must be one executable, with a wrapper script for anything that takes arguments**
  (option C). Every Node, Python or JVM component would need a script per platform.
- **Resolving each argument that looks like a path.** Whether `components/csv.mjs` is a path or a word
  is a guess. Starting the process in the configuration's directory needs no guess.
- **One process per transport instance, enforced by the loader** (finding 18's option A). It contains a
  hung call to one instance, but a component with an expensive start-up, such as the plugin bridge, whose
  plugin is a process of its own, would pay that start-up per mock. Spec §9.3 already leaves the trade to
  the author, and decision 4 tells the engine what the trade cost.
- **Saying nothing about lost instances** (option C alone). A lost mock would answer nothing, and its
  test would fail as if the consumer had never called it.

## Consequences

- `component-config.schema.json`'s `Source` has `args` and `cwd`, both additive. The configuration
  loader (`engine/hooks-host`) resolves `cwd` and a path `reference` for a `subprocess` source.
- `engine/component-host` holds both loaders, WASM and subprocess. The subprocess one needs no runtime
  beyond the standard library's processes and threads, so it adds no dependency.
- `third-party/janus-csv-js` is the worked CSV component written again in JavaScript, with no
  dependencies. It passes `third-party/janus-csv/conformance/`, the Rust component's own cases, through
  the subprocess binding, so the cases hold the component's behaviour and not one implementation of it.
- Tests that need Node skip without it, unless `CI` is set, where they fail: every CI runner has Node.
- **Tripwire**: a component that must keep state across the death of its process, which this binding
  cannot give it without the engine replaying calls it cannot know were idempotent.
