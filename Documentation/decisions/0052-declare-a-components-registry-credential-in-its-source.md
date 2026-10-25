# 0052 — Declare a component's registry credential in its source

- **Status**: accepted
- **Date**: 2026-10-25
- **Plan tasks**: S1.27; X.3 (credentials, cross-cutting security); S1.9 (hook secrets, whose
  redaction this extends)
- **Evidence**: [Phase 9 finding 16](../prototype/phase-9-findings.md#16-registry-credentials-come-from-the-loaders-environment-which-27-says-a-run-must-not-use);
  lifecycle-hooks spec §7.1 and §7.3; component-interfaces spec §10.3; `engine/component-host/src/oci.rs`
  and its tests, against an in-process registry that demands credentials at its token service or as
  a `Basic` challenge; `cli/tests/cli.rs`, a run at `trace` against that registry

## Context

A run should depend on one document. Lifecycle-hooks spec §7.1 puts exactly one transformation
between a project's configuration and the engine: the host's loader interpolates `${VAR}` from the
environment. After that, a run does not depend on the environment of the process that hosts the
engine.

Registry credentials were the exception. The WASM loader read `JANUS_OCI_USERNAME` and
`JANUS_OCI_PASSWORD` itself ([ADR 0021](0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md),
component-interfaces spec §10.3). The argument was that a credential decides *whether* a component can
be fetched, never *which* one runs, since the digest decides that. But two runs of one configuration on
two machines could then differ in whether they start, and nothing in the configuration said why.
Finding 16 named three options: leave it (A); a `source.auth` member, interpolated like a hook's
`run.headers` and redacted like them (B); or read Docker's `~/.docker/config.json` and credential
helpers, as `oras` and `crane` do (C).

The reason not to declare it was that a declaration had no redaction rule. Since S1.3 and S1.9 the
engine has one: a broker descriptor's `auth` member shows its `kind` alone in the frame log, and a
hook's interpolated secrets reach nothing the engine reports. So option B costs little now.

`janus component push` and `pull` also use credentials. They are commands with no configuration, and
they are not runs.

## Decision

1. **A component's `source` takes `auth`, the registry credential**, in the broker descriptor's shape:
   `basic` takes `username` and `password`, `bearer` takes `token`. It is written with `${VAR}`
   references, which the host's loader interpolates as it does every member. A `basic` credential
   answers a `Basic` challenge and is presented to a `Bearer` challenge's token service, which is how
   most registries take one. A `bearer` credential is a registry token, presented as it stands to a
   `Bearer` challenge.
2. **A run's loader presents that credential and no other.** It reads no environment. A registry
   that wants a credential nobody declared refuses as `unauthorized`, and the message says that the
   credential belongs in the component source's `auth`. This is a breaking change: a run that relied
   on the two variables must now declare them, which the error says.
3. **`janus component push` and `pull` keep `JANUS_OCI_USERNAME` and `JANUS_OCI_PASSWORD`.** They have
   no configuration to declare a credential in, and lifecycle-hooks spec §7.1 governs runs. When they
   were given credentials, the declaration they print carries
   `auth: { kind: basic, username: "${JANUS_OCI_USERNAME}", password: "${JANUS_OCI_PASSWORD}" }`. A
   project that pastes it reaches the registry with the same two variables, now named in its
   configuration.
4. **The credential is a secret as lifecycle-hooks spec §7.3 defines one**, and §7.3 now names it. It
   is presented to the registry and appears in nothing else:
   - The frame log shows an `auth` member's `kind` alone, as it does for a broker's.
   - The kernel's `Debug` for it names the kind alone.
   - A malformed credential is refused, by the loader and by the kernel, without being quoted. Serde's
     own message for a string where an object belongs would repeat the string.
   - A refusal from the registry names the reference and the challenge, never what was presented.
5. **A loader refuses a credential it cannot use, before fetching anything:** a kind it cannot present
   (the vocabulary is open), and a credential on a source that is not a registry.
6. **The registry client keeps one accepted authorization per repository and credential**, so two
   declarations naming one repository with different credentials never share one.

## Alternatives considered

- **Leave it (finding 16's option A).** The exception is small and named. But it is the one place a
  run still depends on its host's environment, which X.3 exists to close, and the reason it was taken
  (no redaction rule for a declaration) no longer holds.
- **Keep the variables as a fallback for runs.** Nothing would break. But a run would again depend
  on an environment its configuration does not name, which is the finding.
- **Read Docker's configuration and credential helpers (option C).** It is convenient, and every
  registry user already has it. It is also exactly the ambient dependency §7.1 rules out, and a
  credential helper is a process the engine would run on a configuration's behalf, with no grant.
- **Drop the variables from `push` and `pull` too.** It would be consistent, but those commands would
  then need a credential option on their command line, where a shell's history keeps it.

## Consequences

- A run's configuration says everything it needs to start. Two machines with the same configuration
  and the same named variables agree on whether a component can be fetched.
- Finding 16 is closed, and X.3's credentials row holds for every credential a run uses: broker,
  OAuth2 and registry credentials all come from the resolved configuration.
- Moving to this engine breaks a run that fetched a component from a registry using the two
  variables. The fix is one line in its configuration, and the `unauthorized` refusal says so.
- A credential in an SDK's `components` option crosses the protocol in `consumer-session/create` as
  it does in `verification/verify`, and is redacted the same way. Where it comes from is the SDK
  user's code.
- **Tripwire**: if pilot projects keep registry credentials only in Docker's configuration and the
  `auth` member proves a burden, a `docker-config` credential kind, which the loader resolves from a
  path the configuration names, is the additive answer. It is not an ambient lookup.
