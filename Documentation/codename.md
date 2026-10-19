# The codename, and how it is renamed

"Janus" is a codename. The project's real name is the community's to choose (staged plan §7.1, S0.9),
and that choice is expected late: people give an opinion once they can run their own pipelines on
the real build, not before. By then the codename has reached every surface the project has. This
document is the inventory of those surfaces (staged plan S0.5), and
[ADR 0031](decisions/0031-rename-by-a-rehearsed-rewrite-and-read-persisted-identifiers-under-every-spelling.md)
is the decision behind it. The rename is a command, `.github/scripts/rename.sh <name>`, which CI
rehearses every week (the `Rename rehearsal` workflow), so that when S2.7 runs it for real it is a
reviewed change rather than a search.

## The two rules

**Renamed outright.** Almost every surface. The script rewrites `janus`, `Janus` and `JANUS` to the
new name in every text file, and moves every path that contains it. Where users have written the
old name somewhere the engine does not read back, such as an environment variable in a pipeline, a
package dependency or a binary on a `PATH`, the rename is a breaking change, and its change file is
`major`.

**Dual-spelled.** A *persisted identifier* is a name the engine writes into a document that outlives
it and then reads back from that document: a contract in a broker, a provider shape beside a build,
a component in a registry, a hook script or a verifier configuration in a provider's repository. A
rename cannot simply replace one, because documents recorded before it
still carry the old spelling. So each is defined once, in
[`engine/kernel/src/persisted.rs`](../engine/kernel/src/persisted.rs), as what it *writes*. Its
*earlier* spellings live in [`persisted/earlier.rs`](../engine/kernel/src/persisted/earlier.rs), and
every reader accepts both. The rename moves each identifier's current spelling into `earlier.rs`
before rewriting it, so **the renamed engine writes the new name and reads both**. Nothing published
before the rename has to be republished. An earlier spelling is removed by hand one major version
after the rename that added it.

## Exempt

| What | Why |
|---|---|
| `CHANGELOG.md` | What was released, under the name it was released under. Written by knope, never by hand |
| `engine/kernel/src/persisted/earlier.rs` | It exists to remember the old spellings |
| The repository's name, `rholshausen/pact-janus` (and its Pages site) | Where the project lives is S0.7's decision, not the codename's. A rename that moves the repository is a separate change |

Everything else is renamed, the record included: the ADRs, `Documentation/prototype/` and the
spikes' findings. They cite paths that move, and git keeps the original wording.

## The surfaces

### Persisted identifiers: dual-spelled

| Identifier | Spelling today | Read where |
|---|---|---|
| `CONTRACT_FORMAT`: a contract's `$format` (contract-file spec §2.3, ADR 0011) | `janus-contract/1` | `verification/explain`, verification's sources |
| `PROVIDER_SHAPE_FORMAT`: a provider shape's `$format` (subsumption-check spec §4) | `janus-provider-shape/1` | `read_provider_shape`, `janus check --provider-shape <dir>` |
| `COMPONENT_ARTIFACT_TYPE`: an OCI component's `artifactType` (ADR 0021) | `application/vnd.pact.janus.component.v1` | `janus component pull`, an `oci` component source |
| `COMPONENT_CONFIG_MEDIA_TYPE`: an OCI component's config media type (ADR 0021) | `application/vnd.pact.janus.component.config.v1+json` | the same |
| `COMPONENT_INTERFACE`: the interface a component's world exports (`wit/component.wit`) | `pact:janus-component/pipe@1.0.0` | the WASM loader, for every component it loads |
| `HOOK_SCRIPT_GLOBAL`: the global a scripted hook calls the hook library by (ADR 0015) | `janus` | the script runtime, which binds the library under every spelling |
| `HOOK_BODY_FORMAT`: an http hook's `format` for the hook context as its body (lifecycle-hooks spec §8) | `janus` | the http hook |

**Still to come.** The broker's `specification` value (ADR 0011 decision 6, gate B) is not in the code
yet. When the broker client writes it (stage 1), it is added to `persisted.rs`, as every new persisted
identifier is: one constant in the form the rename reads, an empty list in `earlier.rs`, an entry in
`ALL`, and its readers calling `reads()`.

### Written, never read back: renamed outright

These identify documents the engine writes for people and pipelines. Nothing reads them back to decide
anything, so the rename changes them and its change file says so:

- the subsumption report's `janus-subsumption-report/1` and the compatibility report's
  `janus-compatibility-report/1`;
- the sampling algorithm `janus-ipog-v1`, recorded in a contract's selection report;
- the schema `$id`s under `https://pact.io/janus/` (every spec schema), and the open vocabularies'
  members that spell the codename;
- the SDK conformance suite's `janus-conformance-case/1` and `janus-conformance-report/1`. They pass
  between an SDK and the gates of the same release, so both change together;
- `janus-sdk-behavioural-spec/1`, the behavioural specification's own `$format`, and its
  `session.janus.*` conformance ids;
- the hook library's internal names (`__janus_invoke`, …) and the scripted-hook log target
  `janus::hook::script`.

### Everything else: renamed outright

| Surface | Spelling today | Defined in |
|---|---|---|
| The product's name in prose | Pact Janus, Janus | everywhere: READMEs, specs, ADRs, comments |
| Rust crates | `pact_janus_kernel`, `pact_janus_cli`, … every workspace member, and `janus-csv` | each crate's `Cargo.toml`; the lockfiles follow |
| Binaries | `janus`, `janus-engine`, `janus-sdk-gates` | `cli/Cargo.toml`, `tools/sdk-gates/Cargo.toml` |
| Release asset names | `janus-engine-<triple>`, `janus-<triple>`, `janus-sdk-gates-<triple>`, `janus-engine.wasm`, `janus-spec.tar.gz` | `.github/workflows/release.yml`; ADR 0028 decision 9 and ADR 0030 name them, so renaming them is a superseding ADR |
| The engine's own WIT package | `pact:janus-engine@0.1.0` | `engine/wasm/wit/engine.wit`. It ships in the same release as every host that embeds it |
| The component interface's WIT package | `pact:janus-component@1.0.0` | `Documentation/specs/component-interfaces/wit/component.wit` and its copies. What the loader looks up is dual-spelled, as above |
| The TypeScript SDK | the npm package `pact-janus`, `pact-janus/vitest`; `Janus`, `JanusError`, `useJanus`, … | `sdks/typescript/package.json` and `src/` |
| The JVM SDK | group `io.pact.janus`; packages `io.pact.janus.sdk`, `io.pact.janus.bindings.*`; `Janus`, `JanusOptions`, `JanusExtension`, … | `sdks/jvm/build.gradle.kts`, `settings.gradle.kts`, `sdks/bindings.json` |
| Environment variables | `JANUS_ENGINE`, `JANUS_ENGINE_RELEASE`, `JANUS_ENGINE_CACHE`, `JANUS_SPEC`, `JANUS_CONFORMANCE_REPORT`, `JANUS_COMPONENT_CACHE`, `JANUS_OCI_REGISTRY`, `JANUS_OCI_USERNAME`, `JANUS_OCI_PASSWORD`; for development, `JANUS_CSV_COMPONENT`, `JANUS_ENGINE_WASM`, `JANUS_SDK`, `JANUS_SIGNING_SECRET` and the benchmarks' | where each is read (`git grep 'JANUS_'`) |
| The user cache directory | `pact-janus/components`, `pact-janus/engines` | `engine/component-host/src/oci.rs`, each SDK's engine resolver. A cache refills |
| File-name conventions | `verifier.janus.yaml`; an SDK's `<consumer>-<provider>.janus.json` | the SDKs' `finalise`; examples and docs. Every tool takes these by path, so neither is looked up by name |
| Repository paths | `third-party/janus-csv`, `benchmarks/janus`, `cli/tests/janus-engine-node`, `sdks/jvm/**/io/pact/janus`, `.github/scripts/janus-spec.sh`, … | moved by the script |

The script finds every occurrence. This table is for the people reviewing what the rename means.
Nothing here has to be listed for the script to rename it, and the rehearsal fails if anything is
left behind.

## The new name

One lowercase ASCII word, as "janus" is. The script refuses anything else, because a hyphen or a
second word needs a different spelling on each surface: `pact_new_name_kernel`, `io.pact.newname`,
`NewName`. That is a different rename, and this inventory would grow a column per surface first. The
script also refuses a name the repository already contains, since it could not then show that the
rename is complete.

## Renaming for real (S2.7)

1. A superseding ADR for the release's asset names (ADR 0028 decision 9, ADR 0030), naming the new
   ones. The rest of this inventory needs no decision of its own.
2. On a branch: `.github/scripts/rename.sh <name>`. It records the earlier spellings, rewrites, moves,
   and fails unless nothing is left.
3. `TRYCMD=overwrite cargo test -p pact_<name>_cli --test cmd`. The CLI's help is wrapped to a
   width, so a name of another length breaks its lines at other words; the diff this writes moves
   line breaks and changes no word, as the rehearsal checks.
4. A `major` change file saying what users change: the binaries, the environment variables, the
   packages, the cache directory. Persisted documents are unaffected, which the change file says
   too.
5. Review, merge and release as usual. One major version later, empty `earlier.rs`.

## The rehearsal

`.github/workflows/rename-rehearsal.yml`, weekly, on demand, and on any pull request that changes the
script, the persisted identifiers or this document. It builds the CSV component under the codename,
renames to a placeholder (`rehearsal<run number>`), rewraps the CLI's help snapshots and fails if
that changed more than where their lines break, and then requires the workspace's clippy and
tests, the third-party component, both SDK suites, their conformance, the thinness audit, the binding
pipeline, the protocol client and the demo to pass, renamed. The renamed TypeScript suite loads the
component built before the rename, so the dual-spelling rule is tested against a real artifact. The
kernel's `tests/persisted.rs` reads a contract and a provider shape under each earlier spelling.
Before a rename those lists are empty and the tests check nothing; the rehearsal is what fills them.
