---
engine: minor
---

# A component can be a process the engine starts: the subprocess loader

`janus` and `janus-engine` now load a `subprocess` component source (component-interfaces spec §9.3,
ADR 0108), and `engine/hello` lists `subprocess` among `components.loaders`. A source is
`{ kind: subprocess, reference: <program>, args: [...], cwd: <dir> }`: the program is found on the
engine's `PATH` when it is a bare name and is a path otherwise, its arguments are passed through no
shell, and `cwd` is where it starts. A verifier configuration's loader gives a source with no `cwd`
the configuration file's directory, so relative paths among the arguments mean the same from wherever
a run starts. A declaration an SDK sends without a configuration file is read as written. `args` and
`cwd` are new in `component-config.schema.json`. A `subprocess` source with a `digest`, `auth` or
`signature` is refused.

The process starts with an empty environment plus exactly its `env` grant (and `SystemRoot` on
Windows). Every call has a deadline (`limits.deadline-ms`, 10 seconds by default, plus a poll's own
wait), after which the process is killed (`component-timeout`). A process that exits fails its calls
at once (`component-exited`). The next call starts a new process and handshakes it first, and one that
comes back as another version is `component-malformed`. Out of process the unit of recreation is the
process: the error names the transport instances that went with it as `details.instances`, and a later
call on one of them fails at once with `details.lost: true`. Content, matcher and transport interfaces
are bound; a component that implements `hook` is still refused.

`janus component check --config <file> <name> <cases>...` runs the conformance cases of a component
the configuration declares, of any source, through the binding a run loads it with.
