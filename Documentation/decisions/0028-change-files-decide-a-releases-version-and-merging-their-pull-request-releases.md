# 0028 — Decide a release's version with change files, and release by merging the pull request that consumes them

- **Status**: accepted (decision 9's list of what a release carries is extended by [ADR 0030](0030-a-release-carries-the-sdk-gates-and-the-executable-specification.md); decision 1's one package is three, the engine and each SDK, as the tripwire below foresaw, by [ADR 0060](0060-release-each-sdk-as-files-on-a-github-release-of-its-own-and-fetch-its-pinned-engine.md); its other terms and decisions 2–8 and 10 stand)
- **Date**: 2026-10-12
- **Plan tasks**: staged plan S0.4 (decides it), S0.10 and S0.11 (consume what it publishes), S2.9
  (the SDKs' releases leave with them)
- **Evidence**: `knope.toml`, `.github/workflows/changeset.yml`, `prepare-release.yml` and
  `release.yml`; [ADR 0023](0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)
  decisions 2 and 4 (what a release has to carry);
  [ADR 0027](0027-build-in-the-prototype-repository-and-graduate-sdks-into-their-existing-repositories.md)
  (who reads a release's notes); the commit log to `adf8e2c` (what the commit subjects say)

## Context

S0.4 builds the release pipeline, and before a pipeline there has to be an answer to how a release's
version is decided and what makes one happen.

What is released is one thing. The Rust workspace has a single version, `[workspace.package]`'s, and
it is the one `engine/hello` and `janus version` report. A release is that version as artifacts:
`janus-engine` and `janus` per target triple, the engine as a WASM component, and from S0.11 the
corpora, the conformance suite and the checkers. Nothing goes to crates.io: the compatibility surface
is the protocol, not a Rust API. The two SDKs have versions of their own and pin an engine version
(ADR 0023 decision 4); they are not released from here yet, and at S2.9 they leave.

Three things about this repository bear on the choice:

- **Commit subjects are written for maintainers.** They name plan items: `feat: S0.1 plan grammar v1`
  renamed every plan action, `fix: S0.13 …` changed what the engine refuses, and both pull requests
  merged so far are squash commits typed `chore:`. The type says what kind of work it was. It does
  not say whether someone who pins the engine has to do anything.
- **The readers of a release's notes are in other repositories.** After S2.9 they are the pact-js and
  pact-jvm maintainers deciding whether to move a pin. What they need is written for them, not
  reconstructed from subjects written for us.
- **There are no owners yet** (S0.7 is the community's, and late). Whatever the pipeline needs, it
  cannot need a long-lived credential somebody has to hold.

## Decision

1. **A change that ships is described by a change file.** A pull request that changes what a release
   ships adds a Markdown file under `.changeset/`, in the
   [changesets](https://github.com/changesets/changesets) format: front matter naming the package
   and the size of the change, and a body that is the changelog entry. There is one package, the
   workspace, named `default`.
2. **The size is what it costs someone who pins the engine.** `major`: they must change something to
   move to it — a protocol operation, a frame, a contract or plan document they hold, a CLI flag or
   exit code, or a verdict that was `matched` and no longer is. `minor`: they can do something they
   could not. `patch`: something that was wrong is right. Before 1.0 the numbers move one place
   down, by knope's rule: `major` moves the minor version, and `minor` and `patch` move the patch.
   The protocol, the plan grammar and the contract format keep their own versions, which the engine
   reports and the specifications govern; a change file says so when one of them moves.
3. **Commit subjects are not read**, for a bump or for a changelog line. They keep their Conventional
   Changelog prefixes as a convention for the log.
4. **CI holds pull requests to it** (`changeset.yml`). One that touches the engine's or the CLI's
   sources or manifests, the lockfile, `corpora/` or `conformance/` and adds no change file fails,
   unless its description carries a line `No-Changeset: <why>` — for a change nobody who pins the
   engine could notice. The reason is in the pull request, where a reviewer reads it.
5. **[knope](https://knope.tech) reads the change files**, pinned at one version in the workflows.
   `knope prepare-release` adds them up into the next version, writes it to `Cargo.toml`, lets
   `cargo` bring `Cargo.lock`'s workspace members to it, moves the bodies into `CHANGELOG.md`, and
   deletes the files. A version that is a decision rather than a sum (the first release, 1.0) is
   given to it.
6. **A release is prepared as a pull request and made by merging it.** The `Prepare release` workflow
   runs knope and pushes the result as the `release` branch; a maintainer opens the pull request from
   it, so CI runs on it as on any other. The same can be done by hand. Merging is the decision to
   release.
7. **A release is a version on `main` with no tag.** `release.yml` runs on every push to `main` and
   does nothing unless that is so. Then it builds everything, and only when everything is built and
   attested does it create the tag `v<version>`, at the commit it built, and publish the GitHub
   release with every file attached, in one step. There is never a tag without its artifacts.
8. **No credential outlives a workflow run.** The workflows use the token GitHub gives each run and
   nothing else: no personal access token, no app, no signing key
   ([ADR 0029](0029-attest-what-a-release-built-and-let-an-sdk-verify-the-digest-it-pins.md)). This
   is why a person opens the release pull request: one opened with a run's own token would not have
   CI run on it.
9. **What a release carries, and what it is called.** Bare files, not archives, so an SDK's resolver
   is a download and a hash: `janus-engine-<triple>` and `janus-<triple>` (`.exe` on Windows) for
   `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`,
   `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`; `janus-engine.wasm`; and `SHA256SUMS`.
   "Statically linked" is checked per binary in the workflow and means: on Linux no shared library at
   all (musl); on Windows the C runtime linked in; on macOS, which has no static libc, nothing but
   the libraries the system ships.
10. **The pipeline is rehearsed.** Everything short of publishing runs on any pull request that
    touches the workflow, the manifests, the lockfile or the WASM crate, which includes every release
    pull request.

## Alternatives considered

- **`@changesets/cli`**, the tool the format comes from. It versions `package.json` files and nothing
  else, so the workspace would need a stand-in `package.json`, a script to copy its version into
  `Cargo.toml`, and a Node toolchain at the root of a Rust repository.
- **release-please.** It covers Rust, npm and Java and shows the bump in a pull request, but the bump
  is inferred from commit types, with an explicit one as the override. Here the inference would be
  wrong often enough that the override would be the norm.
- **release-plz.** Built around what is on crates.io and around semver-checking a Rust API. Nothing
  here is published there, and the API is not the compatibility surface.
- **A tool of our own in `tools/`**, as the other checkers are. The format is simple enough, but it
  is code to write and keep for something knope already does. It is the fallback (below).
- **`dist` for the build half.** It owns the workflow file it generates and models binaries with
  installers. The WASM component, the corpora artifact S0.11 adds and the libclang prerequisite are
  outside that, and five matrix legs are short enough to read.
- **A maintainer pushes the tag.** The tag would then exist before its artifacts do, and a build that
  failed would leave a version that resolves to nothing.
- **A bot opens the release pull request**, as changesets' own action does. It needs a token that
  outlives the run for CI to see the pull request, and so a custodian (decision 8).
- **Archives** (`.tar.gz`, `.zip`) per triple. Smaller downloads, and an unpacking step in every
  SDK's resolver, in every language, which is the layer the thinness audit budgets.

## Consequences

Easier: the version is something a reviewer sees decided, one pull request at a time; the release
notes are written by whoever made the change, for whoever has to act on it; and releasing is a button
and a merge, by anyone with write access, with nothing to hand over when owners are named.

Harder: every pull request that touches shipped code has one more file, or a line saying why not.
The path rule is a rule of thumb: a change to a build script could ship and not be caught, and a
comment in `engine/` is caught and ships nothing.

Committed to: the `v<version>` tag and the asset names above, which S0.10's resolvers and every
pinned SDK will depend on, and which join S0.5's inventory of surfaces the codename reaches.

Known gaps:

- The released Linux binaries are musl builds, and the benchmark trend (S0.3) measures a glibc build
  of the same commit. Nobody has measured the difference.
- A release is gated by its pull request's checks, not by `main`'s: the release build starts when the
  merge lands, beside CI, not after it.
- Windows on ARM is not built.

Tripwires:

- **knope stalls or changes direction** (it is pre-1.0, and a small project): the change
  files are plain Markdown and stay; a `tools/` crate replaces the reader.
- **An SDK is released from here before it graduates**: it becomes a second package in `knope.toml`
  with its own change files and a prefixed tag, and this ADR is amended to say how the two relate.
- **Download size matters to an SDK** (each binary is about 28 MB): compress the assets, and take the
  unpacking step into the resolvers knowingly.
- **A long-lived credential becomes available with owners** (S0.7): the release pull request can
  then be opened by the workflow, and decision 8 is revisited rather than quietly dropped.
