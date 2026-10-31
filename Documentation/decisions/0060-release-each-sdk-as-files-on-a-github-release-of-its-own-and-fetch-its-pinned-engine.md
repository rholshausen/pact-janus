# 0060 — Release each SDK as files on a GitHub release of its own, and fetch its pinned engine on first use

- **Status**: accepted
- **Date**: 2026-10-31
- **Plan tasks**: S1.36; amends [ADR 0028](0028-change-files-decide-a-releases-version-and-merging-their-pull-request-releases.md)
  as its tripwire for an SDK released before it graduates asks; moves
  [ADR 0029](0029-attest-what-a-release-built-and-let-an-sdk-verify-the-digest-it-pins.md) decision 3's
  resolver from each SDK's tests into the SDK
- **Evidence**: `sdks/typescript/package.json` (`private`, `0.0.0`) and `sdks/jvm/build.gradle.kts`
  (`0.0.0`), from which nothing is released; `sdks/typescript/src/engine/release.ts`, whose error
  tells a user to download the engine by hand; the resolvers in `sdks/typescript/test/engine-release.ts`
  and `EngineRelease.java`, which only the SDKs' own tests can reach

## Context

Stage 1b is done when a pilot provider in each language has verified from inside its test suite for
a release cycle. Nothing a pilot can depend on exists. The engine is released (S0.4), but the SDKs
are not: each is at `0.0.0`, the TypeScript package is `private`, and a pilot builds one from a
checkout. An SDK built that way still has no engine. It runs `janus-engine` from `JANUS_ENGINE` or
the `PATH`, and when there is none, its error names the release asset to download by hand. The
resolver that fetches the pinned engine and checks its digest exists in both SDKs, as test code.

The SDKs graduate into pact-js and pact-jvm at stage 2's exit (S2.9, ADR 0027), and are published to
npm and Maven Central from there, as those packages' next major version. What is released from here
before then is an incubator's: for pilots, versioned so they can say which one they ran.

npm can publish from a workflow with no stored credential (trusted publishing, which also attests
the package). Maven Central cannot: it needs a portal token and a PGP signing key, both of which
outlive a run, and ADR 0028 decision 8 forbids that until S0.7 names owners to hold them.

## Decision

1. **An SDK is released from this repository as files on a GitHub release of its own**, until it
   graduates. Nothing is published to a registry.
2. **Each SDK is a knope package beside the engine**, as ADR 0028's tripwire said it would be:
   `typescript-sdk` (its version in `sdks/typescript/package.json`) and `jvm-sdk` (in
   `sdks/jvm/gradle.properties`), each with its own `CHANGELOG.md` beside it. The engine's package is
   `engine`. A change file names the packages it moves, and one change may move several.
3. **Its tag is `<package>/v<version>`**: `typescript-sdk/v0.1.0`, `jvm-sdk/v0.1.0`. The engine's tag
   stays `v<version>` (ADR 0030). An SDK's version says nothing about the engine's; the engine it
   runs is its pin (decision 6).
4. **What a release carries.** For the TypeScript SDK, `pact-janus-<version>.tgz`, which is
   `npm pack`'s output, installed by its URL:
   `npm install https://github.com/<repository>/releases/download/typescript-sdk/v<version>/pact-janus-<version>.tgz`.
   The package stays `private`, so it cannot be published to npm by mistake. For the JVM SDK, the
   `janus-sdk` and `janus-bindings` modules (group `io.pact.janus`), each as its jar, its sources
   jar, its POM and its Ivy descriptor. Gradle reads them in place, through an `ivy` repository whose
   pattern is the release's download URL. Maven installs them with `install:install-file` and the
   POM. Both releases carry a `SHA256SUMS`, and every file is attested, as the engine's are (ADR 0029).
5. **A change that ships in an SDK carries a change file naming that SDK**: its sources, its manifest
   and lockfile, its generated bindings and its engine pin. CI's `Change file` workflow holds pull
   requests to it, as it does for the engine. The pull request that moves `sdks/engine-release.json`
   to a new engine is one, so `release.yml` writes onto the `engine-pin` branch, with the pin, a
   change file for each SDK. For an SDK whose release is waiting for that pin (decision 7), it writes
   a line in that release's changelog entry instead.
6. **An SDK fetches its pinned engine on first use.** The pin it was built with
   (`sdks/engine-release.json`) ships inside it. Unless it is told which executable to run, by its
   own option or `JANUS_ENGINE`, it runs the pinned engine for its platform. It takes the engine from
   the cache (`JANUS_ENGINE_CACHE`, or `pact-janus/engines` in the user's cache directory) when it is
   there with the pinned digest. Otherwise it downloads the engine from the pinned release, or from
   `JANUS_ENGINE_MIRROR` when that names another base URL, and refuses any file whose digest is not
   the pinned one. It no longer looks on the `PATH`: an SDK runs the engine it was released with, or
   the one it is named. The resolver moves from each SDK's tests into its `embedding` layer, whose
   budget grows by it.
7. **A release proves itself before it is published**, as the engine's does (ADR 0028 decision 7).
   The new `release-sdks.yml` runs on every push to `main`, and finds each SDK whose version has no
   tag. For each, it builds, runs the SDK's suite against the pinned engine with no Rust toolchain
   (S0.10), and installs what it is about to publish into a project outside the repository. That
   project resolves the package from the staged files as a user resolves it from the release, sets no
   `JANUS_ENGINE`, and runs a consumer test and a provider test. Only then does the workflow tag and
   publish. It rehearses all of this, short of publishing, on any pull request that touches it, an
   SDK's manifest or the engine pin.
   *Amended 2026-10-31*, by what S1.37's first rehearsal showed:
   - **An SDK is released against the engine the tree is at.** When a release pull request also
     releases the engine, Cargo.toml's version is ahead of the pin, and the SDK waits. Building it
     against the old pin fails any change that needs the new engine. Merging the pull request that
     pins the new engine is the push that releases it.
   - **Each SDK is released on its own** (`release-sdk.yml`, called once for each), so one that
     fails stops only its own release.
8. **ADR 0028 decision 8 stands.** A GitHub release needs nothing but the run's own token.

## Alternatives considered

- **npm and Maven Central now.** These are where a user expects to find a package, and npm's trusted
  publishing needs no stored credential. Central does, which decision 8 forbids until S0.7, and
  publishing a 0.x under a name pact-js and pact-jvm will take at S2.9 makes two packages to retire.
  Either SDK can go there before graduating under the tripwire below.
- **GitHub Packages.** It needs no new credential to publish, but a consumer has to authenticate to
  read even a public package, in npm and in Maven, which a pilot's CI has to be set up for.
- **A Maven repository on GitHub Pages.** It gives Maven and Gradle a real repository URL, and needs
  only the run's token to deploy. But it is a second place a release lives, with its own history to
  keep, for two modules. Gradle already reads the release in place, and Maven users can install two
  files.
- **Per-platform engine packages** (npm `optionalDependencies`, as esbuild ships its binaries;
  classifier jars). These resolve through a registry mirror, which a locked-down network allows and
  a GitHub download often is not. But with no registry, there is nothing to mirror, and
  `JANUS_ENGINE_MIRROR` serves the same need.
- **The engine inside the package.** That puts all five binaries in every install, about 140 MB.
- **Lockstep versions**, one for the engine and both SDKs. Every engine release would republish both
  SDKs unchanged, and every SDK fix would release an engine. The pin already says which engine an SDK
  runs.
- **Keep the `PATH` fallback after the pin.** A `janus-engine` on the `PATH` is whatever version was
  installed last. A user who wants it can name it in `JANUS_ENGINE`.

## Consequences

Easier: a pilot depends on a version, by URL, and gets the engine it was tested with, with no
toolchain and no manual download. A pilot's report names a version we can check out.

Harder: three packages release from one repository, and one release pull request may release any of
them. The first use of an SDK on a machine downloads about 28 MB from GitHub. An offline or
locked-down CI needs the cache primed, a mirror, or `JANUS_ENGINE`. Node's `fetch` does not use
`HTTPS_PROXY` unless `NODE_USE_ENV_PROXY` is set, and the JVM uses its own proxy properties. Both are
documented, and neither is worked around. Maven users install by hand.

Committed to: the tags `typescript-sdk/v<version>` and `jvm-sdk/v<version>`, the asset names in
decision 4, and the `io.pact.janus` coordinates, which join S0.5's inventory of names the codename
reaches. Also `JANUS_ENGINE_CACHE` and `JANUS_ENGINE_MIRROR`, which until now only the tests read.

Tripwires:

- **A pilot cannot reasonably consume a release this way** (a Maven build with no shell step, or a
  network that reaches registries and not GitHub): that SDK moves to its registry. For Maven Central,
  decision 8 is amended to allow a registry credential and a signing key as secrets of a protected
  environment, used only by the publish job, behind an approval, and held by the owners S0.7 names.
- **S2.9 graduates an SDK**: its releases here stop, its package leaves `knope.toml`, and this ADR is
  superseded for it.
