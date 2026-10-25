---
default: major
---

# A component's registry credential is declared in its source, and a run reads no environment for it

A component fetched from an OCI registry used to be fetched with `JANUS_OCI_USERNAME` and
`JANUS_OCI_PASSWORD` from the environment of whatever hosted the engine. So two runs of one configuration
could differ in whether they started, and nothing in the configuration said why (ADR 0052).

- **A component declaration's `source` takes `auth`**, in the broker descriptor's shape: `basic` with
  `username` and `password`, or `bearer` with a registry `token`. A host's loader interpolates `${VAR}` in
  it as in any member, and `hooks-host`'s loader does. A `basic` credential answers a `Basic` challenge and
  is presented to a `Bearer` challenge's token service; a `bearer` one is presented as it stands.
- **Breaking: a run presents that credential and no other.** `verification/verify`, `verification/explain`
  and `consumer-session/create` no longer read the two variables. A registry that wants a credential
  nobody declared refuses with `unauthorized`, inside `component-unavailable`, and the message says the
  credential belongs in the component source's `auth`. A configuration that relied on the variables adds
  `auth: { kind: basic, username: "${JANUS_OCI_USERNAME}", password: "${JANUS_OCI_PASSWORD}" }`.
- **Refused before anything is fetched:** an `auth` whose kind the loader cannot present, and an `auth`
  on a source that is not a registry. A malformed `auth` (not an object, a member that is not a string,
  `basic` without a password, `bearer` without a token) is an invalid declaration, and the message never
  quotes it.
- **The credential is never repeated**: the frame log shows an `auth` member's `kind` alone, as for a
  broker, and no error, event or summary carries it.
- **`janus component push` and `pull` still read the two variables**, having no configuration. When given
  them, the declaration they print carries the `auth` above.

`pact_janus_component_host`'s API follows: `Oci::new` takes no credentials, `Oci::pull`/`push` and
`WasmLoader::pull`/`push` take an `Option<&Credentials>`, and `Credentials` is an enum (`Basic`,
`Bearer`) with `from_declared`.
