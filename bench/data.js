window.BENCHMARK_DATA = {
  "lastUpdate": 1791844017039,
  "repoUrl": "https://github.com/rholshausen/pact-janus",
  "entries": {
    "Janus — time and size (lower is better)": [
      {
        "commit": {
          "author": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "497fc5388b8888461b587d87abe2b943f12effc7",
          "message": "chore: S0.3 benchmark trend in CI (#2)\n\n* chore: S0.3 the benchmark trend in CI\n\nStaged plan item S0.3. A new Benchmarks workflow runs the 1.7 scenarios\nagainst Janus on every merge to main, in all three embeddings, and adds\nthe numbers to a chart published from the gh-pages branch:\n\n  https://rholshausen.github.io/pact-janus/bench/\n\nA trend, not a gate: nothing fails for being slower. The runners are\nshared machines, and a single point means little.\n\n- The harness gains `trend <results-dir> <out-dir>`, which turns result\n  documents into the points the chart plots: one line per number worth\n  watching, from a table in trend.rs. A test over the checked-in results\n  fails when a scenario reaches no line. JANUS_BENCH_RESULTS_DIR says\n  where a run's documents go.\n- The points are recorded by benchmark-action/github-action-benchmark\n  v1.22.2, pinned by commit because its job holds a token that can\n  write. Only a push to main records. A pull request that touches the\n  harness or the workflow measures and compares, and records nothing.\n- The whole result documents are an artifact of each run.\n- The harness's own tests run here too. It is outside the root\n  workspace, so nothing else in CI built or tested it.\n\nAlso three result documents from the machine the 2026-09-24 points came\nfrom, at fce02a0: the first comparable points since the prototype, after\nS0.1 and S0.13 changed the kernel.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n\n* fix: the second recording step when nothing is recorded\n\nOn a pull request the first call commits to the runner's own gh-pages and\ndoes not push, so the second call's fetch was refused as a non-fast-forward.\nIt skips the fetch when not recording.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n\n---------\n\nCo-authored-by: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-11T16:03:00+11:00",
          "tree_id": "ab11f8e9c029101459a803d63e4d40990d18fc28",
          "url": "https://github.com/rholshausen/pact-janus/commit/497fc5388b8888461b587d87abe2b943f12effc7"
        },
        "date": 1791695600997,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 6.9,
            "unit": "µs",
            "extra": "min 6.7 µs, p95 9.8 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1778.2,
            "unit": "µs",
            "extra": "min 1738.9 µs, p95 1876.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 169.1,
            "unit": "µs",
            "extra": "min 96.6 µs, p95 575.7 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1688.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6899625,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 28363.6,
            "unit": "µs",
            "extra": "min 27830 µs, p95 28783.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 29692.8,
            "unit": "µs",
            "extra": "min 29284 µs, p95 30317.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 33394.2,
            "unit": "µs",
            "extra": "min 33218.3 µs, p95 33606.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 334.3,
            "unit": "µs",
            "extra": "min 323.3 µs, p95 347.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 428.7,
            "unit": "µs",
            "extra": "min 398.3 µs, p95 445.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 354.5,
            "unit": "µs",
            "extra": "min 333 µs, p95 378.9 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13307.2,
            "unit": "µs",
            "extra": "min 11693.3 µs, p95 13506.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 14220.2,
            "unit": "µs",
            "extra": "min 14029.9 µs, p95 14373.2 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 14.9,
            "unit": "µs",
            "extra": "min 14.5 µs, p95 15.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.6,
            "unit": "µs",
            "extra": "min 20.1 µs, p95 21 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 18040.3,
            "unit": "µs",
            "extra": "min 17578.9 µs, p95 18342.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 18420.6,
            "unit": "µs",
            "extra": "min 17389.6 µs, p95 22039 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 9975.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 10143.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 77.5,
            "unit": "µs",
            "extra": "min 67.9 µs, p95 88.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 107.1,
            "unit": "µs",
            "extra": "min 97.5 µs, p95 120 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 13.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 38.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 84.2,
            "unit": "µs",
            "extra": "min 68.7 µs, p95 96.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 104.1,
            "unit": "µs",
            "extra": "min 91.8 µs, p95 117.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 17.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 38.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 265.5,
            "unit": "µs",
            "extra": "min 235.6 µs, p95 358.6 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 373.5,
            "unit": "µs",
            "extra": "min 306.6 µs, p95 506.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 27682.5,
            "unit": "µs",
            "extra": "min 27381.4 µs, p95 27895.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 28890.7,
            "unit": "µs",
            "extra": "min 28572.8 µs, p95 29124.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 33902.4,
            "unit": "µs",
            "extra": "min 33649.7 µs, p95 34130 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 207.2,
            "unit": "µs",
            "extra": "min 201 µs, p95 220.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 254.8,
            "unit": "µs",
            "extra": "min 245.3 µs, p95 305.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 300.1,
            "unit": "µs",
            "extra": "min 290.7 µs, p95 319.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.6,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.9,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.24,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.27,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.127,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.155,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 4.98,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.693,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 20.13,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 21.02,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.3,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.3,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.4,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.41,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "adf8e2c16b502d00d254d212f2db432d2215ae9b",
          "message": "docs: S0.3 is done; its first point is recorded and the chart is live\n\nThe Done line for staged plan item S0.3, written after the exit could be\nchecked rather than before: the merge that added the Benchmarks workflow\n(497fc53) recorded 51 numbers to gh-pages, and the chart at\nhttps://rholshausen.github.io/pact-janus/bench/ draws them.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-11T16:18:16+11:00",
          "tree_id": "02221ce5279659173756546bbc47524c7326f5b7",
          "url": "https://github.com/rholshausen/pact-janus/commit/adf8e2c16b502d00d254d212f2db432d2215ae9b"
        },
        "date": 1791696153438,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.3,
            "unit": "µs",
            "extra": "min 8 µs, p95 11 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1502.7,
            "unit": "µs",
            "extra": "min 1437.4 µs, p95 1774.8 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 144.6,
            "unit": "µs",
            "extra": "min 105.5 µs, p95 261.1 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2236.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6899625,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 35582.5,
            "unit": "µs",
            "extra": "min 34337.7 µs, p95 36748 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36987.9,
            "unit": "µs",
            "extra": "min 35952.8 µs, p95 37961.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 43004.4,
            "unit": "µs",
            "extra": "min 42359.4 µs, p95 43573.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 439.7,
            "unit": "µs",
            "extra": "min 420.9 µs, p95 456.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 483.5,
            "unit": "µs",
            "extra": "min 456.2 µs, p95 547.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 504,
            "unit": "µs",
            "extra": "min 481.9 µs, p95 519.7 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18680.8,
            "unit": "µs",
            "extra": "min 17517.5 µs, p95 19628.1 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18966.6,
            "unit": "µs",
            "extra": "min 18171.8 µs, p95 19519.9 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.3,
            "unit": "µs",
            "extra": "min 18.6 µs, p95 19.6 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26.3,
            "unit": "µs",
            "extra": "min 25.4 µs, p95 26.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 28734,
            "unit": "µs",
            "extra": "min 26287.4 µs, p95 30251.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 29840.1,
            "unit": "µs",
            "extra": "min 28036.8 µs, p95 31469.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 17076.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 17738,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 114.9,
            "unit": "µs",
            "extra": "min 105.8 µs, p95 126.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 128.4,
            "unit": "µs",
            "extra": "min 116.9 µs, p95 144.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 26.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 48.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 97.8,
            "unit": "µs",
            "extra": "min 86.9 µs, p95 113.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 123.3,
            "unit": "µs",
            "extra": "min 110.5 µs, p95 141.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 19,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 47.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 295.8,
            "unit": "µs",
            "extra": "min 215.6 µs, p95 396 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 442.2,
            "unit": "µs",
            "extra": "min 377.4 µs, p95 552.6 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34285.5,
            "unit": "µs",
            "extra": "min 33380.4 µs, p95 35091.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 36112.1,
            "unit": "µs",
            "extra": "min 35041.7 µs, p95 36927.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 43219.4,
            "unit": "µs",
            "extra": "min 42468 µs, p95 43853.5 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 291.4,
            "unit": "µs",
            "extra": "min 278.6 µs, p95 314.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 339.1,
            "unit": "µs",
            "extra": "min 320.1 µs, p95 359.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 402.2,
            "unit": "µs",
            "extra": "min 384 µs, p95 424.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.94,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.21,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.28,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.31,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.16,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.18,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 5.4,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.731,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 27.2,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.57,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.33,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.32,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.45,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.49,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "8e96e0538d5c32614bb0855341f24c646e217c26",
          "message": "chore: S0.4 the release pipeline\n\nStaged plan item S0.4, short of its exit: nothing is released yet. This\nis everything a release needs, and the first one follows from it.\n\nHow a release is decided (ADR 0028). The version and the changelog come\nfrom change files under .changeset/, in the changesets format, and not\nfrom commit subjects. knope 0.23.0 reads them. A pull request that\nchanges what a release ships adds one, or says why not with a\n\"No-Changeset: <why>\" line in its description; the Change file workflow\nholds pull requests to that. A release is prepared as a pull request\n(the Prepare release workflow, or knope by hand) and made by merging it.\nNo step needs a credential that outlives a workflow run.\n\nWhat a release is (release.yml). janus-engine and janus, statically\nlinked, for five target triples; the engine as a WASM component; and a\nSHA256SUMS. Each binary is checked for dynamic dependencies and run\nbefore it is staged. Only when everything is built and attested is the\nv<version> tag created, at the commit that was built, with the release.\nEverything short of publishing runs on pull requests that touch the\nworkflow, the manifests, the lockfile or the WASM crate.\n\nHow it is signed (ADR 0029). Build provenance attestations, keyless,\nover every file; an SDK verifies the SHA-256 it pins beside the version.\n\n- The WASM engine moves from benchmarks/janus/engine-wasm to\n  engine/wasm, as a workspace member that is empty on any target but\n  wasm32. The world a release ships exports the pipe alone. The\n  benchmark-only interface is behind a `bench` feature the harness\n  builds with. New tests load the component under wasmtime, and CI\n  lints both worlds for wasm32-wasip2.\n- sdks/engine-release.json is the pin an SDK resolves a released engine\n  from: a version and a digest per triple, empty until there is a first\n  release. The TypeScript suite resolves it with JANUS_ENGINE_RELEASE=1\n  and builds from the tree otherwise. A download whose digest is not the\n  pinned one is refused and not kept.\n- S0.5's inventory gains the WIT package names, the release asset names\n  and the repository a pin names.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-12T17:26:04+11:00",
          "tree_id": "c0328870b8f55920de35d68403d44aba5f324cdc",
          "url": "https://github.com/rholshausen/pact-janus/commit/8e96e0538d5c32614bb0855341f24c646e217c26"
        },
        "date": 1791786952063,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 5.7,
            "unit": "µs",
            "extra": "min 5.4 µs, p95 7.2 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1616.1,
            "unit": "µs",
            "extra": "min 1400.4 µs, p95 1759.4 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 92.7,
            "unit": "µs",
            "extra": "min 83.1 µs, p95 109.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1525.5,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6898086,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 23907,
            "unit": "µs",
            "extra": "min 23327.5 µs, p95 27342.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 24583.1,
            "unit": "µs",
            "extra": "min 24122.3 µs, p95 28311.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 26176.2,
            "unit": "µs",
            "extra": "min 25881.9 µs, p95 26443.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 314.4,
            "unit": "µs",
            "extra": "min 302.2 µs, p95 327.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 410.3,
            "unit": "µs",
            "extra": "min 387.1 µs, p95 449.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 328.7,
            "unit": "µs",
            "extra": "min 310.9 µs, p95 341.3 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13262.2,
            "unit": "µs",
            "extra": "min 12821.7 µs, p95 13467.3 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 11625,
            "unit": "µs",
            "extra": "min 11394.6 µs, p95 12935.7 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 13.5,
            "unit": "µs",
            "extra": "min 12.9 µs, p95 13.8 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 16.5,
            "unit": "µs",
            "extra": "min 15.8 µs, p95 17.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 19336.1,
            "unit": "µs",
            "extra": "min 18308.7 µs, p95 22558.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 19342.1,
            "unit": "µs",
            "extra": "min 18155.6 µs, p95 22821.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 11306.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 10947.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 81.4,
            "unit": "µs",
            "extra": "min 74.8 µs, p95 93.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 101.4,
            "unit": "µs",
            "extra": "min 87.2 µs, p95 131.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 16.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 38.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 104.9,
            "unit": "µs",
            "extra": "min 89.2 µs, p95 114.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 152.9,
            "unit": "µs",
            "extra": "min 110.2 µs, p95 164.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 16.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 65.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 222.5,
            "unit": "µs",
            "extra": "min 186.6 µs, p95 315 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 800.2,
            "unit": "µs",
            "extra": "min 286.6 µs, p95 1047236.6 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 22898.7,
            "unit": "µs",
            "extra": "min 22563.9 µs, p95 26179.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 23715.9,
            "unit": "µs",
            "extra": "min 23289.2 µs, p95 24155.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 29287.1,
            "unit": "µs",
            "extra": "min 25611.6 µs, p95 30155.6 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 200.9,
            "unit": "µs",
            "extra": "min 192.7 µs, p95 213.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 273.2,
            "unit": "µs",
            "extra": "min 264.4 µs, p95 287.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 271.7,
            "unit": "µs",
            "extra": "min 261.7 µs, p95 282.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.68,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.19,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.22,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.28,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.141,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.188,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 4.29,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.568,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 19.18,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 22.77,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.25,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.27,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.38,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.42,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "d3d4c85f4d61c7840f95d4e75a6a9151ac778335",
          "message": "fix: a release moves the benchmark harness's lockfile too\n\nThe Benchmarks run on the 0.1.0 release commit failed at the step that\nrecords its point. benchmarks/janus is a workspace of its own that\ndepends on five of this workspace's crates by path, so its Cargo.lock\nnames them at a version. `knope prepare-release` refreshed the root\nlockfile and not that one. The harness's build then rewrote it in place,\nand with the checkout dirty the benchmark action could not switch to\ngh-pages.\n\n- benchmarks/janus/Cargo.lock names the five crates at 0.1.0. Nothing\n  else in it moves.\n- knope.toml refreshes and stages it when a release is prepared.\n- CI's checks job resolves, with --locked, every lockfile outside the\n  workspace that names one of its crates. It runs on every pull\n  request, so a release that leaves one behind fails before it merges.\n- The harness's tests build with --locked, so a stale lockfile fails\n  there, by name.\n\nThe release commit's point on the trend is not recorded.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-12T18:39:12+11:00",
          "tree_id": "e33219a656ba45e2ba679c20275a889b136c41d7",
          "url": "https://github.com/rholshausen/pact-janus/commit/d3d4c85f4d61c7840f95d4e75a6a9151ac778335"
        },
        "date": 1791791572048,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.8,
            "unit": "µs",
            "extra": "min 8.5 µs, p95 11.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2140.8,
            "unit": "µs",
            "extra": "min 2092 µs, p95 2241.9 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 189.3,
            "unit": "µs",
            "extra": "min 134.2 µs, p95 311.1 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2155.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6897811,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 36853.6,
            "unit": "µs",
            "extra": "min 35849.7 µs, p95 37912.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 38628,
            "unit": "µs",
            "extra": "min 38052.3 µs, p95 39293.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 44706,
            "unit": "µs",
            "extra": "min 44130.9 µs, p95 45070.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 435.2,
            "unit": "µs",
            "extra": "min 418.6 µs, p95 452.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 501.7,
            "unit": "µs",
            "extra": "min 452 µs, p95 564.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 463.9,
            "unit": "µs",
            "extra": "min 435.1 µs, p95 491.3 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17685.1,
            "unit": "µs",
            "extra": "min 16923.3 µs, p95 18323.8 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18081.4,
            "unit": "µs",
            "extra": "min 17755.4 µs, p95 18369.6 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.8,
            "unit": "µs",
            "extra": "min 19.2 µs, p95 20.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26.2,
            "unit": "µs",
            "extra": "min 25.6 µs, p95 27 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 25059.3,
            "unit": "µs",
            "extra": "min 22849.1 µs, p95 25965.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 25624.4,
            "unit": "µs",
            "extra": "min 23853.8 µs, p95 31119.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 14065.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 14388.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 130.2,
            "unit": "µs",
            "extra": "min 94.8 µs, p95 151.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 133.9,
            "unit": "µs",
            "extra": "min 124.1 µs, p95 155.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 26.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 46.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 96,
            "unit": "µs",
            "extra": "min 87.5 µs, p95 113.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 129,
            "unit": "µs",
            "extra": "min 113.4 µs, p95 146.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 17.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 47.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 369.8,
            "unit": "µs",
            "extra": "min 256.6 µs, p95 461.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 483.5,
            "unit": "µs",
            "extra": "min 377.2 µs, p95 569.9 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 35937,
            "unit": "µs",
            "extra": "min 35512.1 µs, p95 36409.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 37610.6,
            "unit": "µs",
            "extra": "min 36937.5 µs, p95 38242.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 45515.8,
            "unit": "µs",
            "extra": "min 44843.5 µs, p95 46110.4 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 274.8,
            "unit": "µs",
            "extra": "min 264.5 µs, p95 297.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 329.2,
            "unit": "µs",
            "extra": "min 318.5 µs, p95 355 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 390.8,
            "unit": "µs",
            "extra": "min 374 µs, p95 405.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.05,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.44,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.31,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.35,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.164,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.199,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.36,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.879,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 27.64,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.53,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.38,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.4,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.54,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.55,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "d6f20b08144b4db8bfd466b4de2071aea9938162",
          "message": "docs: S0.4's record gains what its first release broke\n\nThe Benchmarks run on the release commit failed: the harness's lockfile\nnames workspace crates at a version, and preparing the release left it\nbehind. The fix is its own pull request (#6); this is the line in the\nplan that says the exit did not foresee it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-12T18:42:14+11:00",
          "tree_id": "d6b584553aba514be325bff82912a47658ab7c71",
          "url": "https://github.com/rholshausen/pact-janus/commit/d6f20b08144b4db8bfd466b4de2071aea9938162"
        },
        "date": 1791791840349,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 16.7,
            "unit": "µs",
            "extra": "min 14.5 µs, p95 41.1 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1981.5,
            "unit": "µs",
            "extra": "min 1926.1 µs, p95 2150.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 273.7,
            "unit": "µs",
            "extra": "min 206.2 µs, p95 308.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2220.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6897811,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33382.1,
            "unit": "µs",
            "extra": "min 33009.9 µs, p95 34396.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35230,
            "unit": "µs",
            "extra": "min 34700.6 µs, p95 36449.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40432.7,
            "unit": "µs",
            "extra": "min 40095.4 µs, p95 41637.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 421.8,
            "unit": "µs",
            "extra": "min 407.6 µs, p95 439.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 527.5,
            "unit": "µs",
            "extra": "min 476 µs, p95 558.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 481.7,
            "unit": "µs",
            "extra": "min 454.9 µs, p95 540.2 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16224.6,
            "unit": "µs",
            "extra": "min 15835.4 µs, p95 16495.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18554.1,
            "unit": "µs",
            "extra": "min 18354.9 µs, p95 19042.7 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.5,
            "unit": "µs",
            "extra": "min 18.9 µs, p95 20 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.9,
            "unit": "µs",
            "extra": "min 25.5 µs, p95 26.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22200,
            "unit": "µs",
            "extra": "min 21544.6 µs, p95 24098.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 22291.5,
            "unit": "µs",
            "extra": "min 21786.6 µs, p95 28255.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 11842,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 11906.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 135.4,
            "unit": "µs",
            "extra": "min 126.4 µs, p95 140.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 173.5,
            "unit": "µs",
            "extra": "min 149.8 µs, p95 184.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 22.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 62.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 141.5,
            "unit": "µs",
            "extra": "min 123.8 µs, p95 146.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 173.3,
            "unit": "µs",
            "extra": "min 144.2 µs, p95 182.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 26.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 63.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 369.7,
            "unit": "µs",
            "extra": "min 279.7 µs, p95 495.8 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 511.3,
            "unit": "µs",
            "extra": "min 411 µs, p95 663 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33097.1,
            "unit": "µs",
            "extra": "min 32608.1 µs, p95 34079.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34178.8,
            "unit": "µs",
            "extra": "min 33766.6 µs, p95 34914.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40335.9,
            "unit": "µs",
            "extra": "min 40021.3 µs, p95 41447.1 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 277,
            "unit": "µs",
            "extra": "min 270.5 µs, p95 299.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 412.4,
            "unit": "µs",
            "extra": "min 351.5 µs, p95 432.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 399.6,
            "unit": "µs",
            "extra": "min 388.1 µs, p95 419.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.18,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.51,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.31,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.33,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.182,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.207,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.42,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.914,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 24.23,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 26.08,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.37,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.4,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.5,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.52,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "76dfd22216f03637f643b33a4ec73bd8a19ffb55",
          "message": "docs: Update CONTRIBUTING.md with release command examples",
          "timestamp": "2026-10-13T09:23:39+11:00",
          "tree_id": "6b7bdf44b063b8b3600af267b08d73f77855ec21",
          "url": "https://github.com/rholshausen/pact-janus/commit/76dfd22216f03637f643b33a4ec73bd8a19ffb55"
        },
        "date": 1791844014565,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 4.8,
            "unit": "µs",
            "extra": "min 4.6 µs, p95 6.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1499.7,
            "unit": "µs",
            "extra": "min 1444.7 µs, p95 1577.3 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 115.6,
            "unit": "µs",
            "extra": "min 82 µs, p95 148.7 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1267.6,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6897811,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 23067,
            "unit": "µs",
            "extra": "min 21908.9 µs, p95 25019.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 24086.6,
            "unit": "µs",
            "extra": "min 23178.1 µs, p95 25403.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 24682.5,
            "unit": "µs",
            "extra": "min 23610.5 µs, p95 26283.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 243.9,
            "unit": "µs",
            "extra": "min 237.6 µs, p95 253 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 292.9,
            "unit": "µs",
            "extra": "min 256.2 µs, p95 310.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 236.6,
            "unit": "µs",
            "extra": "min 226.7 µs, p95 249.4 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 10846.2,
            "unit": "µs",
            "extra": "min 10206.4 µs, p95 13056.3 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 10169.5,
            "unit": "µs",
            "extra": "min 10047.1 µs, p95 10637.8 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 11.4,
            "unit": "µs",
            "extra": "min 11.2 µs, p95 11.7 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 14.1,
            "unit": "µs",
            "extra": "min 13.8 µs, p95 14.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 18451.4,
            "unit": "µs",
            "extra": "min 15311.3 µs, p95 20858.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 17017.8,
            "unit": "µs",
            "extra": "min 14403.5 µs, p95 20933 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 10632,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 9859.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 57.1,
            "unit": "µs",
            "extra": "min 54.9 µs, p95 65.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 69.5,
            "unit": "µs",
            "extra": "min 59.2 µs, p95 77.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 9.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 24.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 54.6,
            "unit": "µs",
            "extra": "min 47 µs, p95 59.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 71.1,
            "unit": "µs",
            "extra": "min 57.6 µs, p95 78 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 9.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 27,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 204.3,
            "unit": "µs",
            "extra": "min 180.2 µs, p95 274.7 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 272.5,
            "unit": "µs",
            "extra": "min 226.1 µs, p95 311.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 22685.6,
            "unit": "µs",
            "extra": "min 21873.5 µs, p95 23700.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 22908.7,
            "unit": "µs",
            "extra": "min 21934.5 µs, p95 24136.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 24548.2,
            "unit": "µs",
            "extra": "min 23705.2 µs, p95 25415.5 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 159.7,
            "unit": "µs",
            "extra": "min 155.8 µs, p95 172.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 187.7,
            "unit": "µs",
            "extra": "min 181.9 µs, p95 197.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 214.1,
            "unit": "µs",
            "extra": "min 209.6 µs, p95 226 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.16,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.32,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.19,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.21,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.086,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.096,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 3.65,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.489,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 19.24,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 18.47,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.21,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.22,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.31,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.31,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          }
        ]
      }
    ],
    "Janus — throughput (higher is better)": [
      {
        "commit": {
          "author": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "497fc5388b8888461b587d87abe2b943f12effc7",
          "message": "chore: S0.3 benchmark trend in CI (#2)\n\n* chore: S0.3 the benchmark trend in CI\n\nStaged plan item S0.3. A new Benchmarks workflow runs the 1.7 scenarios\nagainst Janus on every merge to main, in all three embeddings, and adds\nthe numbers to a chart published from the gh-pages branch:\n\n  https://rholshausen.github.io/pact-janus/bench/\n\nA trend, not a gate: nothing fails for being slower. The runners are\nshared machines, and a single point means little.\n\n- The harness gains `trend <results-dir> <out-dir>`, which turns result\n  documents into the points the chart plots: one line per number worth\n  watching, from a table in trend.rs. A test over the checked-in results\n  fails when a scenario reaches no line. JANUS_BENCH_RESULTS_DIR says\n  where a run's documents go.\n- The points are recorded by benchmark-action/github-action-benchmark\n  v1.22.2, pinned by commit because its job holds a token that can\n  write. Only a push to main records. A pull request that touches the\n  harness or the workflow measures and compares, and records nothing.\n- The whole result documents are an artifact of each run.\n- The harness's own tests run here too. It is outside the root\n  workspace, so nothing else in CI built or tested it.\n\nAlso three result documents from the machine the 2026-09-24 points came\nfrom, at fce02a0: the first comparable points since the prototype, after\nS0.1 and S0.13 changed the kernel.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n\n* fix: the second recording step when nothing is recorded\n\nOn a pull request the first call commits to the runner's own gh-pages and\ndoes not push, so the second call's fetch was refused as a non-fast-forward.\nIt skips the fetch when not recording.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n\n---------\n\nCo-authored-by: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-11T16:03:00+11:00",
          "tree_id": "ab11f8e9c029101459a803d63e4d40990d18fc28",
          "url": "https://github.com/rholshausen/pact-janus/commit/497fc5388b8888461b587d87abe2b943f12effc7"
        },
        "date": 1791695603284,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 40891,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 30205,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "adf8e2c16b502d00d254d212f2db432d2215ae9b",
          "message": "docs: S0.3 is done; its first point is recorded and the chart is live\n\nThe Done line for staged plan item S0.3, written after the exit could be\nchecked rather than before: the merge that added the Benchmarks workflow\n(497fc53) recorded 51 numbers to gh-pages, and the chart at\nhttps://rholshausen.github.io/pact-janus/bench/ draws them.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-11T16:18:16+11:00",
          "tree_id": "02221ce5279659173756546bbc47524c7326f5b7",
          "url": "https://github.com/rholshausen/pact-janus/commit/adf8e2c16b502d00d254d212f2db432d2215ae9b"
        },
        "date": 1791696155594,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 36597,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 30346,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "8e96e0538d5c32614bb0855341f24c646e217c26",
          "message": "chore: S0.4 the release pipeline\n\nStaged plan item S0.4, short of its exit: nothing is released yet. This\nis everything a release needs, and the first one follows from it.\n\nHow a release is decided (ADR 0028). The version and the changelog come\nfrom change files under .changeset/, in the changesets format, and not\nfrom commit subjects. knope 0.23.0 reads them. A pull request that\nchanges what a release ships adds one, or says why not with a\n\"No-Changeset: <why>\" line in its description; the Change file workflow\nholds pull requests to that. A release is prepared as a pull request\n(the Prepare release workflow, or knope by hand) and made by merging it.\nNo step needs a credential that outlives a workflow run.\n\nWhat a release is (release.yml). janus-engine and janus, statically\nlinked, for five target triples; the engine as a WASM component; and a\nSHA256SUMS. Each binary is checked for dynamic dependencies and run\nbefore it is staged. Only when everything is built and attested is the\nv<version> tag created, at the commit that was built, with the release.\nEverything short of publishing runs on pull requests that touch the\nworkflow, the manifests, the lockfile or the WASM crate.\n\nHow it is signed (ADR 0029). Build provenance attestations, keyless,\nover every file; an SDK verifies the SHA-256 it pins beside the version.\n\n- The WASM engine moves from benchmarks/janus/engine-wasm to\n  engine/wasm, as a workspace member that is empty on any target but\n  wasm32. The world a release ships exports the pipe alone. The\n  benchmark-only interface is behind a `bench` feature the harness\n  builds with. New tests load the component under wasmtime, and CI\n  lints both worlds for wasm32-wasip2.\n- sdks/engine-release.json is the pin an SDK resolves a released engine\n  from: a version and a digest per triple, empty until there is a first\n  release. The TypeScript suite resolves it with JANUS_ENGINE_RELEASE=1\n  and builds from the tree otherwise. A download whose digest is not the\n  pinned one is refused and not kept.\n- S0.5's inventory gains the WIT package names, the release asset names\n  and the repository a pin names.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-12T17:26:04+11:00",
          "tree_id": "c0328870b8f55920de35d68403d44aba5f324cdc",
          "url": "https://github.com/rholshausen/pact-janus/commit/8e96e0538d5c32614bb0855341f24c646e217c26"
        },
        "date": 1791786954223,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 66476,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 47835,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "d3d4c85f4d61c7840f95d4e75a6a9151ac778335",
          "message": "fix: a release moves the benchmark harness's lockfile too\n\nThe Benchmarks run on the 0.1.0 release commit failed at the step that\nrecords its point. benchmarks/janus is a workspace of its own that\ndepends on five of this workspace's crates by path, so its Cargo.lock\nnames them at a version. `knope prepare-release` refreshed the root\nlockfile and not that one. The harness's build then rewrote it in place,\nand with the checkout dirty the benchmark action could not switch to\ngh-pages.\n\n- benchmarks/janus/Cargo.lock names the five crates at 0.1.0. Nothing\n  else in it moves.\n- knope.toml refreshes and stages it when a release is prepared.\n- CI's checks job resolves, with --locked, every lockfile outside the\n  workspace that names one of its crates. It runs on every pull\n  request, so a release that leaves one behind fails before it merges.\n- The harness's tests build with --locked, so a stale lockfile fails\n  there, by name.\n\nThe release commit's point on the trend is not recorded.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-12T18:39:12+11:00",
          "tree_id": "e33219a656ba45e2ba679c20275a889b136c41d7",
          "url": "https://github.com/rholshausen/pact-janus/commit/d3d4c85f4d61c7840f95d4e75a6a9151ac778335"
        },
        "date": 1791791573979,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 32111,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 23498,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "d6f20b08144b4db8bfd466b4de2071aea9938162",
          "message": "docs: S0.4's record gains what its first release broke\n\nThe Benchmarks run on the release commit failed: the harness's lockfile\nnames workspace crates at a version, and preparing the release left it\nbehind. The fix is its own pull request (#6); this is the line in the\nplan that says the exit did not foresee it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-12T18:42:14+11:00",
          "tree_id": "d6b584553aba514be325bff82912a47658ab7c71",
          "url": "https://github.com/rholshausen/pact-janus/commit/d6f20b08144b4db8bfd466b4de2071aea9938162"
        },
        "date": 1791791842916,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28178,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21821,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "committer": {
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "76dfd22216f03637f643b33a4ec73bd8a19ffb55",
          "message": "docs: Update CONTRIBUTING.md with release command examples",
          "timestamp": "2026-10-13T09:23:39+11:00",
          "tree_id": "6b7bdf44b063b8b3600af267b08d73f77855ec21",
          "url": "https://github.com/rholshausen/pact-janus/commit/76dfd22216f03637f643b33a4ec73bd8a19ffb55"
        },
        "date": 1791844016586,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 66686,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 54655,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      }
    ]
  }
}