window.BENCHMARK_DATA = {
  "lastUpdate": 1792371030991,
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
          "id": "83267ed5cf373ff3e08f1f02dcfcb58af57648a9",
          "message": "fix: write the CLI tests' shared consumer contract once, not once per test\n\nSix tests in cli/tests/cli.rs each rewrote the same temporary contract file,\nand they run in parallel. `std::fs::write` truncates before it writes, so a\n`janus check` one test had started could read the file empty — \"EOF while\nparsing a value at line 1 column 0\", seen on CI's Linux runner. Latent since\ntask 7.4; the file is now written once per test process.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-16T15:40:41+11:00",
          "tree_id": "e98e661825d4ff0b7e0bff9eaf0864b496eb8f5e",
          "url": "https://github.com/rholshausen/pact-janus/commit/83267ed5cf373ff3e08f1f02dcfcb58af57648a9"
        },
        "date": 1792126438509,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.9,
            "unit": "µs",
            "extra": "min 8.5 µs, p95 39.6 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1987.1,
            "unit": "µs",
            "extra": "min 1930 µs, p95 2069.1 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 184.7,
            "unit": "µs",
            "extra": "min 158.1 µs, p95 1128.4 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2222.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6898134,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33688.4,
            "unit": "µs",
            "extra": "min 33266.4 µs, p95 34572.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35959.8,
            "unit": "µs",
            "extra": "min 35333.3 µs, p95 36677.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40573.8,
            "unit": "µs",
            "extra": "min 40207 µs, p95 40915.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 428.6,
            "unit": "µs",
            "extra": "min 410.2 µs, p95 453 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 533.4,
            "unit": "µs",
            "extra": "min 474.1 µs, p95 572 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 478.1,
            "unit": "µs",
            "extra": "min 455.6 µs, p95 498.6 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16305,
            "unit": "µs",
            "extra": "min 15886.6 µs, p95 17027.3 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18413.2,
            "unit": "µs",
            "extra": "min 18157.6 µs, p95 19022.4 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.2,
            "unit": "µs",
            "extra": "min 18.7 µs, p95 19.8 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.5,
            "unit": "µs",
            "extra": "min 25.2 µs, p95 26.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22698.3,
            "unit": "µs",
            "extra": "min 21813.4 µs, p95 24262.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 23680,
            "unit": "µs",
            "extra": "min 21994.3 µs, p95 28980.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12236.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12904.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 139.9,
            "unit": "µs",
            "extra": "min 129.9 µs, p95 152.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 177.5,
            "unit": "µs",
            "extra": "min 163.4 µs, p95 189.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 23.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 63.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 134,
            "unit": "µs",
            "extra": "min 124.3 µs, p95 147.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 178.7,
            "unit": "µs",
            "extra": "min 154.7 µs, p95 193 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 21.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 64.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 374.4,
            "unit": "µs",
            "extra": "min 253.3 µs, p95 495.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 496.8,
            "unit": "µs",
            "extra": "min 415.1 µs, p95 638.5 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33367.8,
            "unit": "µs",
            "extra": "min 32679.8 µs, p95 33963.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35225.5,
            "unit": "µs",
            "extra": "min 34541.7 µs, p95 35684.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40821.5,
            "unit": "µs",
            "extra": "min 40415.6 µs, p95 41121.6 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 286.7,
            "unit": "µs",
            "extra": "min 274.5 µs, p95 313.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 423.2,
            "unit": "µs",
            "extra": "min 366.1 µs, p95 454.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 404.7,
            "unit": "µs",
            "extra": "min 389.2 µs, p95 425.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.2,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.63,
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
            "value": 0.34,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.183,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.22,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.41,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.895,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 24.46,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.76,
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
            "value": 0.41,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.51,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.53,
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
          "id": "741588470b75856ddb5cc5824db77823d28e8c29",
          "message": "feat: S0.11 the SDK gates and the executable specification ship with the engine\n\nEvery release now carries janus-sdk-gates-<triple> for the five triples and\njanus-spec.tar.gz (ADR 0030, extending ADR 0028 decision 9): 17 files. A\ncheckout holding one SDK directory runs its conformance, thinness and\nbindings gates from those files alone, which release.yml's new sdk-gates job\nproves on every rehearsal and before every publish.\n\n- tools/conformance, tools/thinness and tools/bindings move their commands\n  into their libraries and take --spec <dir> (a checkout or an unpacked\n  janus-spec.tar.gz) in place of the source path compiled into them;\n  tools/sdk-gates is the three as one binary.\n- janus-spec.tar.gz is the corpora, the conformance suite, the behavioural\n  specification, sdks/bindings.json and its schema sets, at their paths here,\n  made by `git archive` (.github/scripts/janus-spec.sh).\n- Each SDK keeps its own thinness.json, with paths relative to it; with none\n  named, the audit measures every sdks/*/thinness.json side by side.\n- Both SDKs' drivers read JANUS_SPEC for the conformance cases and schemas.\n- The Change file workflow counts the gates' sources and the specification's\n  files as shipped.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-16T16:31:07+11:00",
          "tree_id": "79355d29920c7010cd839b6f8d89e7be7af684b7",
          "url": "https://github.com/rholshausen/pact-janus/commit/741588470b75856ddb5cc5824db77823d28e8c29"
        },
        "date": 1792129483074,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.5,
            "unit": "µs",
            "extra": "min 8.2 µs, p95 11.7 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1920.1,
            "unit": "µs",
            "extra": "min 1870.8 µs, p95 2045.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 281.5,
            "unit": "µs",
            "extra": "min 188.3 µs, p95 808.8 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2198.2,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6898220,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33523.7,
            "unit": "µs",
            "extra": "min 33071.3 µs, p95 34566.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36078.6,
            "unit": "µs",
            "extra": "min 35151.8 µs, p95 37095.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40814.7,
            "unit": "µs",
            "extra": "min 39993 µs, p95 41232.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 425.8,
            "unit": "µs",
            "extra": "min 411.4 µs, p95 445.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 531.8,
            "unit": "µs",
            "extra": "min 472.6 µs, p95 553.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 475.6,
            "unit": "µs",
            "extra": "min 449.1 µs, p95 495.8 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16410.4,
            "unit": "µs",
            "extra": "min 15975.4 µs, p95 17405.2 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18354.4,
            "unit": "µs",
            "extra": "min 18155.6 µs, p95 18744.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19,
            "unit": "µs",
            "extra": "min 18.5 µs, p95 19.4 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.4,
            "unit": "µs",
            "extra": "min 25 µs, p95 26.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 23139.8,
            "unit": "µs",
            "extra": "min 21905.7 µs, p95 24169.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 23475.6,
            "unit": "µs",
            "extra": "min 21736.4 µs, p95 28953.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12503.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12774.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 132.2,
            "unit": "µs",
            "extra": "min 123.7 µs, p95 141.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 171.7,
            "unit": "µs",
            "extra": "min 150 µs, p95 179.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 21.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 58.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 133.6,
            "unit": "µs",
            "extra": "min 109.9 µs, p95 148.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 167.7,
            "unit": "µs",
            "extra": "min 142.8 µs, p95 177.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 61.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 385.2,
            "unit": "µs",
            "extra": "min 294.9 µs, p95 509 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 509.6,
            "unit": "µs",
            "extra": "min 443 µs, p95 617.8 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33088.5,
            "unit": "µs",
            "extra": "min 32561.4 µs, p95 33638.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34790.9,
            "unit": "µs",
            "extra": "min 33990.6 µs, p95 38357.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40758.3,
            "unit": "µs",
            "extra": "min 40328.9 µs, p95 41163.7 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 276.2,
            "unit": "µs",
            "extra": "min 269.2 µs, p95 297.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 408.1,
            "unit": "µs",
            "extra": "min 347.5 µs, p95 437.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 398.9,
            "unit": "µs",
            "extra": "min 387.9 µs, p95 419.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.15,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.47,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.3,
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
            "value": 0.178,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.202,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.39,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.922,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.23,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.93,
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
            "value": 0.39,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.51,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.54,
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
          "id": "7ffba9f5f29cbd8cda33e67c24d3c6dca106fd11",
          "message": "feat: S0.5 rename-ready under the codename\n\nThe codename is renamed by one command, `.github/scripts/rename.sh <name>`,\nand the `Rename rehearsal` workflow runs it every week on a throwaway\ncheckout (ADR 0031; the inventory is Documentation/codename.md).\n\n- The script rewrites janus/Janus/JANUS in every text file, the record\n  included, moves every path that contains it, and fails unless nothing is\n  left but CHANGELOG.md, the earlier spellings and the repository's own name\n  (S0.7's). It refuses a name that is not one lowercase word, or that the\n  repository already contains.\n- Persisted identifiers, names the engine reads back from documents that\n  outlive it, are engine/kernel/src/persisted.rs: each written under one\n  spelling and read under every earlier one, which the script records in\n  persisted/earlier.rs before it rewrites. Seven of them: a contract's and a\n  provider shape's $format, the OCI component artifact and config types, the\n  interface a component exports, the hook script global and the http hook's\n  body format. Their readers now call reads().\n- The rehearsal builds a component before renaming and loads it after, and\n  requires clippy, every test, both SDK suites and their conformance,\n  thinness, the binding pipeline, the protocol client and the demo to pass,\n  renamed. kernel tests/persisted.rs reads documents under each earlier\n  spelling, which only a rename provides.\n\nStage 0a is done.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-16T18:51:34+11:00",
          "tree_id": "5d7c4d7cdeeca76476415849e057bc5eee0828cc",
          "url": "https://github.com/rholshausen/pact-janus/commit/7ffba9f5f29cbd8cda33e67c24d3c6dca106fd11"
        },
        "date": 1792137947209,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 14.3,
            "unit": "µs",
            "extra": "min 8.7 µs, p95 18.7 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1994.9,
            "unit": "µs",
            "extra": "min 1944 µs, p95 2070.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 222.9,
            "unit": "µs",
            "extra": "min 154.1 µs, p95 395.1 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2287.5,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6898847,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34556.2,
            "unit": "µs",
            "extra": "min 32915.2 µs, p95 36831.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 37239.5,
            "unit": "µs",
            "extra": "min 35072.6 µs, p95 39540.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41087.7,
            "unit": "µs",
            "extra": "min 40331.7 µs, p95 42603.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 430.3,
            "unit": "µs",
            "extra": "min 412.3 µs, p95 453.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 536.5,
            "unit": "µs",
            "extra": "min 476.1 µs, p95 576.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 478.1,
            "unit": "µs",
            "extra": "min 452.2 µs, p95 511.4 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 19062.6,
            "unit": "µs",
            "extra": "min 16888.1 µs, p95 21364.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19907,
            "unit": "µs",
            "extra": "min 18686.1 µs, p95 20912.3 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.2,
            "unit": "µs",
            "extra": "min 18.7 µs, p95 19.8 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26,
            "unit": "µs",
            "extra": "min 25.7 µs, p95 26.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 25398.8,
            "unit": "µs",
            "extra": "min 22522.2 µs, p95 29090.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 26978.5,
            "unit": "µs",
            "extra": "min 24369.4 µs, p95 31805.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13855.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 15161,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 138.8,
            "unit": "µs",
            "extra": "min 127.8 µs, p95 149.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 179.8,
            "unit": "µs",
            "extra": "min 151.4 µs, p95 189.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 22.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 66,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 135.4,
            "unit": "µs",
            "extra": "min 119.3 µs, p95 146.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 171.4,
            "unit": "µs",
            "extra": "min 143.3 µs, p95 188.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 61.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 410.1,
            "unit": "µs",
            "extra": "min 322.4 µs, p95 484.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 475,
            "unit": "µs",
            "extra": "min 391.7 µs, p95 607 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33562.1,
            "unit": "µs",
            "extra": "min 32630 µs, p95 34940.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 36226,
            "unit": "µs",
            "extra": "min 34150.3 µs, p95 38321.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41132.4,
            "unit": "µs",
            "extra": "min 40258.6 µs, p95 42657.7 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 286.3,
            "unit": "µs",
            "extra": "min 271.3 µs, p95 321.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 415.2,
            "unit": "µs",
            "extra": "min 401.4 µs, p95 444.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 406.3,
            "unit": "µs",
            "extra": "min 388.1 µs, p95 440.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.25,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.49,
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
            "value": 0.34,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.185,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.201,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.78,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.974,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 28.58,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 27.19,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.42,
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
            "value": 0.53,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "e6fff87c364b45e453fde7206dbddc896bfd4653",
          "message": "feat: the janus CLI rebuilt on clap, ready for stage 1a\n\nThe CLI was built for the prototype, with a hand-rolled argument parser.\nBefore stage 1a puts it in front of providers, it is a proper command line:\none clap declaration (cli/src/args.rs) is the flags, the help, the usage\nerrors and their exit code.\n\nBreaking, with a change file (major): --json is now --output text|json on\nverify, check, upgrade and component push/pull, the spelling the Pact Broker\nclient uses, leaving --json <file> free for pact_verifier_cli parity (S1.13).\nupgrade --out is now --write, so it cannot be confused with --output.\n\nFixed:\n- janus verify gave up when a poll waited 10s with no events, so a slow\n  provider or state hook failed a run that was still going. It now waits for\n  the terminal event. That needed a kernel fix first: a verification run that\n  panicked never ended its stream, and the 10s cut-off was hiding a hang. A\n  run now always ends with verification/finished, status failed and an\n  internal error (engine-protocol spec §9.6; its own change file, patch).\n- A closed stdout (janus ... | head) panicked in println!. Output goes\n  through outln!/errln!, which drop writes to a gone stream; the command runs\n  to its end and keeps its exit code.\n- Mistakes clap now refuses that were ignored: extra arguments, values that\n  are not warn|block, an --as-of that is not a calendar date, a non-numeric\n  --index, and flags that cannot be combined.\n- check --output json dropped the \"no provider shape\" warning on stderr.\n- janus-engine --version waited on stdin; it now answers --version and\n  --help. A Content-Length larger than the body sent is an EOF error, not an\n  allocation of the claimed size.\n\nAlso: one error type instead of ~30 hand-formatted failures; the commands\nin a library target, so both binaries share it without #[path]; help text\nwritten for users; --log-level, janus --version, janus completions <shell>;\ncoloured logs only on a terminal without NO_COLOR; the duplicated directory\nwalk in io.rs folded into one.\n\nNew dependencies: clap and clap_complete; trycmd and snapbox for tests.\nNone of rstest, expectest or pretty_assertions snapshots a command, so\nCLAUDE.md now says how these are used: cli/tests/cmd/ holds trycmd cases\n(every --help in help.md, the usage errors, offline commands), and\ncli/tests/cli.rs compares whole outputs with snapbox where a provider or\nregistry must run first. Engine-owned text is elided; corpora/ pins it.\n\nThe filtered-run snapshot shows a gap left alone here: a --variant that\nmatches nothing reports VERIFIED and exits 0. Staged plan S1.17 records it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T11:11:25+11:00",
          "tree_id": "8066214780a5c6953f5361cbb81b47641d4b5081",
          "url": "https://github.com/rholshausen/pact-janus/commit/e6fff87c364b45e453fde7206dbddc896bfd4653"
        },
        "date": 1792282743969,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 14.3,
            "unit": "µs",
            "extra": "min 8.8 µs, p95 39.8 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2104.1,
            "unit": "µs",
            "extra": "min 2000.6 µs, p95 2237 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 243.1,
            "unit": "µs",
            "extra": "min 193.2 µs, p95 319.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2269,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6881985,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33983.1,
            "unit": "µs",
            "extra": "min 32994.9 µs, p95 35366.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36226.7,
            "unit": "µs",
            "extra": "min 34433.9 µs, p95 37470.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40718.7,
            "unit": "µs",
            "extra": "min 39899.8 µs, p95 41248.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 429.4,
            "unit": "µs",
            "extra": "min 408.8 µs, p95 450.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 522.2,
            "unit": "µs",
            "extra": "min 467 µs, p95 566.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 477.5,
            "unit": "µs",
            "extra": "min 455.2 µs, p95 501.6 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18206.4,
            "unit": "µs",
            "extra": "min 15722.2 µs, p95 20285.8 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19307.4,
            "unit": "µs",
            "extra": "min 18196.7 µs, p95 20074.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 18.6,
            "unit": "µs",
            "extra": "min 18.2 µs, p95 19 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26,
            "unit": "µs",
            "extra": "min 25.5 µs, p95 28 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 25601.1,
            "unit": "µs",
            "extra": "min 21769.7 µs, p95 27957.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 25610.1,
            "unit": "µs",
            "extra": "min 21901.8 µs, p95 31593 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 14307.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 14311.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 135,
            "unit": "µs",
            "extra": "min 128.3 µs, p95 150.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 179.6,
            "unit": "µs",
            "extra": "min 153.4 µs, p95 194.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 21.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 64.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 133.2,
            "unit": "µs",
            "extra": "min 119.2 µs, p95 145.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 173.7,
            "unit": "µs",
            "extra": "min 144.5 µs, p95 191.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 64.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 374.9,
            "unit": "µs",
            "extra": "min 280.5 µs, p95 526.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 467,
            "unit": "µs",
            "extra": "min 408.5 µs, p95 603.2 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33528,
            "unit": "µs",
            "extra": "min 32501 µs, p95 34446.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35124.6,
            "unit": "µs",
            "extra": "min 33624.3 µs, p95 37126.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40645.1,
            "unit": "µs",
            "extra": "min 39960 µs, p95 41431.1 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 282.1,
            "unit": "µs",
            "extra": "min 272.5 µs, p95 313.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 414,
            "unit": "µs",
            "extra": "min 349.7 µs, p95 448.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 400.4,
            "unit": "µs",
            "extra": "min 386.7 µs, p95 419.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.23,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.64,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.3,
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
            "value": 0.188,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.219,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.41,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.904,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 26.14,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 27.24,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.4,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.41,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.53,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.57,
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
          "id": "1ee0beb8cb2b7de74054e225980ddf811b0b4626",
          "message": "fix: help and usage errors name the program janus on Windows too\n\nclap takes the program's name from argv[0], which on Windows is janus.exe,\nso every Usage: line said `janus.exe ...` there and the trycmd snapshots of\nhelp and usage errors failed on the windows-latest job. The name is now\npinned with bin_name, in janus and in janus-engine. Reproduced and checked\non Linux by running copies of both binaries named *.exe.\n\nThe unreleased CLI change file gains a line for it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T11:35:02+11:00",
          "tree_id": "f7a739fabdef00de748d13649b746091cde05323",
          "url": "https://github.com/rholshausen/pact-janus/commit/1ee0beb8cb2b7de74054e225980ddf811b0b4626"
        },
        "date": 1792284238991,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 7.5,
            "unit": "µs",
            "extra": "min 7.1 µs, p95 10.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1770.9,
            "unit": "µs",
            "extra": "min 1699 µs, p95 1881.3 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 196,
            "unit": "µs",
            "extra": "min 127.4 µs, p95 254.8 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2141.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6881985,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33190.6,
            "unit": "µs",
            "extra": "min 32559.2 µs, p95 34142.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 34714.3,
            "unit": "µs",
            "extra": "min 33657.3 µs, p95 36190.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 36293.9,
            "unit": "µs",
            "extra": "min 35311.5 µs, p95 37341.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 452.8,
            "unit": "µs",
            "extra": "min 429.2 µs, p95 469.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 485.8,
            "unit": "µs",
            "extra": "min 460.6 µs, p95 538 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 479.7,
            "unit": "µs",
            "extra": "min 453.7 µs, p95 497.3 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17978.4,
            "unit": "µs",
            "extra": "min 17529.7 µs, p95 18424 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 15761.8,
            "unit": "µs",
            "extra": "min 15549.1 µs, p95 15921.8 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.4,
            "unit": "µs",
            "extra": "min 18.7 µs, p95 20.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 22.8,
            "unit": "µs",
            "extra": "min 21.9 µs, p95 24.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 27535.5,
            "unit": "µs",
            "extra": "min 23778.7 µs, p95 30187.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 27263.3,
            "unit": "µs",
            "extra": "min 24474 µs, p95 29360.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 16407.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 15961.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 107.2,
            "unit": "µs",
            "extra": "min 94.3 µs, p95 123.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 138,
            "unit": "µs",
            "extra": "min 110.8 µs, p95 161.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 31.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 55,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 100.7,
            "unit": "µs",
            "extra": "min 85 µs, p95 117.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 128.3,
            "unit": "µs",
            "extra": "min 105.5 µs, p95 155.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 54.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 305.9,
            "unit": "µs",
            "extra": "min 229.7 µs, p95 413.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 449.7,
            "unit": "µs",
            "extra": "min 280.8 µs, p95 594.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 32432.4,
            "unit": "µs",
            "extra": "min 31590.3 µs, p95 33583.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 33378.6,
            "unit": "µs",
            "extra": "min 32315 µs, p95 34916.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 35668.2,
            "unit": "µs",
            "extra": "min 35047.6 µs, p95 36540.1 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 294.8,
            "unit": "µs",
            "extra": "min 277 µs, p95 311.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 352.3,
            "unit": "µs",
            "extra": "min 329.4 µs, p95 392.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 382.3,
            "unit": "µs",
            "extra": "min 366.7 µs, p95 394.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.04,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.36,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.29,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.36,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.167,
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
            "value": 5.42,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.721,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 26.81,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.1,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.31,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.31,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.46,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.47,
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
          "id": "30e7234152d51261dc18c31daa03274e24f978f9",
          "message": "feat: S1.1 the differential verification harness and its fixture corpus\n\n\"Exactly as today\" is now a number. tools/verifier-diff runs today's\nverifier (pact_verifier_cli, pinned at 1.3.5 in\ntools/verifier-diff/pact-verifier.json) and janus verify on every pact in\ncorpora/verifier/ against the same provider, and compares the verdicts per\ninteraction and for the whole run. CI's new verifier-diff job runs it,\ninstalling the pinned verifier with .github/scripts/pact-verifier.sh, which\nrefuses any other digest.\n\n- The corpus: the order-service's v3 pact against the order-service itself;\n  the pact-compatibility-suite's HTTP provider scenarios V1-V4 (Apache-2.0,\n  commit 1acfa1e), each pact built with pact_models as the suite's own\n  harness builds it, 29 imported and 5 skipped with the plan item that lets\n  them in (S1.3 broker, S1.9 request filter); and deterministic mutations of\n  each interaction's response. 96 cases, 194 verdicts, in about two minutes.\n- The scripted provider recognises requests strictly by example, as pact's\n  mock server does, answers a fixture's routes, and takes provider states.\n- The ledger, corpora/verifier/disagreements.json, lists every disagreement\n  with the plan item or ADR that accounts for it. The run fails on one it\n  does not list, and on a listed one that has gone. Today: 177 agree, 17\n  known, nothing unexpected.\n\nWhat the first run found, in the plan:\n- S1.18 (new, decided): today's verifiers match a regex against the whole\n  value (Oniguruma's is_match in pact-reference, Kotlin's String.matches in\n  pact-jvm); the shape-language spec says v1-v4 match anywhere. The legacy\n  compiler and upgrade will anchor; shapes keep matching anywhere.\n- S1.6 gains plain-text bodies, which are component-unavailable today.\n- S1.9 notes that today's verifier retries a failing state call for about\n  two minutes and the http hook does not retry.\n- S3.3 keeps list-valued headers in stage 3, deliberately, with why.\n\njanus verify's summary lists failures but not passes, so passes are inferred\nfrom its counts, cross-checked; S1.3 needs per-variant results in it anyway.\ncargo test checks that every fixture and ledger entry reads, without a\nverifier installed. corpora/ ships in janus-spec.tar.gz, hence the change\nfile.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T12:44:44+11:00",
          "tree_id": "5c60919078d362a66bc68d36f1747a6f534fd429",
          "url": "https://github.com/rholshausen/pact-janus/commit/30e7234152d51261dc18c31daa03274e24f978f9"
        },
        "date": 1792288588567,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.5,
            "unit": "µs",
            "extra": "min 8.2 µs, p95 10.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2065.4,
            "unit": "µs",
            "extra": "min 2007.5 µs, p95 2121.2 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 250.5,
            "unit": "µs",
            "extra": "min 168.9 µs, p95 1672 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2214.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6881985,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33739,
            "unit": "µs",
            "extra": "min 33071.8 µs, p95 34349.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35535.9,
            "unit": "µs",
            "extra": "min 34752.7 µs, p95 37080.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40617.4,
            "unit": "µs",
            "extra": "min 40245.7 µs, p95 40982.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 427.2,
            "unit": "µs",
            "extra": "min 408.4 µs, p95 453.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 507.6,
            "unit": "µs",
            "extra": "min 464 µs, p95 554.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 482.8,
            "unit": "µs",
            "extra": "min 454 µs, p95 522 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16250.8,
            "unit": "µs",
            "extra": "min 15828.2 µs, p95 17199.4 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19266.8,
            "unit": "µs",
            "extra": "min 18530.5 µs, p95 20322.5 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 18.7,
            "unit": "µs",
            "extra": "min 18.3 µs, p95 19 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.7,
            "unit": "µs",
            "extra": "min 25.3 µs, p95 26.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22812.5,
            "unit": "µs",
            "extra": "min 21754.1 µs, p95 24500.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 23494.9,
            "unit": "µs",
            "extra": "min 21977.8 µs, p95 29390.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12473.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12687.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 133,
            "unit": "µs",
            "extra": "min 117.6 µs, p95 138.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 180.7,
            "unit": "µs",
            "extra": "min 143.7 µs, p95 195.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 20.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 64.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 135,
            "unit": "µs",
            "extra": "min 115.2 µs, p95 147.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 172.3,
            "unit": "µs",
            "extra": "min 141.6 µs, p95 183.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 64.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 402.6,
            "unit": "µs",
            "extra": "min 265 µs, p95 753.8 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 513.7,
            "unit": "µs",
            "extra": "min 455.1 µs, p95 701.8 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33037.4,
            "unit": "µs",
            "extra": "min 32627.1 µs, p95 33554.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34867.2,
            "unit": "µs",
            "extra": "min 34024 µs, p95 35973.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40909.4,
            "unit": "µs",
            "extra": "min 40468.6 µs, p95 41456 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 279.8,
            "unit": "µs",
            "extra": "min 271.2 µs, p95 304.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 361.7,
            "unit": "µs",
            "extra": "min 333.8 µs, p95 392 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 404.2,
            "unit": "µs",
            "extra": "min 385.7 µs, p95 431.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.19,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.52,
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
            "value": 0.34,
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
            "value": 0.2,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.35,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.897,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.04,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.95,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.41,
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
            "value": 0.51,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.56,
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
          "id": "60d1a8305581c72dc809869482a2207a01f4e6f5",
          "message": "docs: one set of agent instructions, in AGENTS.md\n\nCLAUDE.md and .github/copilot-instructions.md said the same things twice and\nhad drifted apart in structure, so every change was two edits that did not\nalways apply. AGENTS.md is now the only copy: Claude Code reads it through\nCLAUDE.md, which is just `@AGENTS.md`, and GitHub Copilot (the coding agent\nand PR review) and other agents read it directly.\n\n- CLAUDE.md moves to AGENTS.md, history kept. Its opening says who reads it.\n- .github/copilot-instructions.md is deleted. Compared sentence by sentence,\n  it held nothing CLAUDE.md lacked, and one stale line (`pact.execute`).\n- CONTRIBUTING.md and the live comments and specs that cite the rules now\n  cite AGENTS.md. The prototype record, the ADRs and the finished S0.5 entry\n  are left as written; CLAUDE.md still resolves for them.\n- The status paragraph records S1.1 done: its CI job passed on its first\n  run, with every disagreement in the ledger.\n\nNo-Changeset: comments and documentation only; nothing a release ships changes behaviour.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T13:01:35+11:00",
          "tree_id": "bb3ffcc3d68d4f6130340d3737e31eac18f6d60b",
          "url": "https://github.com/rholshausen/pact-janus/commit/60d1a8305581c72dc809869482a2207a01f4e6f5"
        },
        "date": 1792289134348,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 9.3,
            "unit": "µs",
            "extra": "min 6.8 µs, p95 12.2 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1846.6,
            "unit": "µs",
            "extra": "min 1795.1 µs, p95 1926.5 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 139.2,
            "unit": "µs",
            "extra": "min 110.9 µs, p95 238.7 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1724.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6881985,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 29224.7,
            "unit": "µs",
            "extra": "min 28570.3 µs, p95 31873.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 30571.9,
            "unit": "µs",
            "extra": "min 29894.1 µs, p95 31484.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 34334.9,
            "unit": "µs",
            "extra": "min 33997.1 µs, p95 35735.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 344.5,
            "unit": "µs",
            "extra": "min 329.5 µs, p95 363.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 384.6,
            "unit": "µs",
            "extra": "min 357 µs, p95 405.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 360.9,
            "unit": "µs",
            "extra": "min 341.1 µs, p95 394 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13315.2,
            "unit": "µs",
            "extra": "min 12850.7 µs, p95 14147 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 13967.2,
            "unit": "µs",
            "extra": "min 13733.5 µs, p95 14670.8 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 15.1,
            "unit": "µs",
            "extra": "min 14.5 µs, p95 15.8 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.2,
            "unit": "µs",
            "extra": "min 19.6 µs, p95 20.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 18931.8,
            "unit": "µs",
            "extra": "min 18086.7 µs, p95 21032.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 19064.1,
            "unit": "µs",
            "extra": "min 18423.5 µs, p95 21015.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 10577.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 10632.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 77.9,
            "unit": "µs",
            "extra": "min 72.7 µs, p95 89.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 106.6,
            "unit": "µs",
            "extra": "min 93.8 µs, p95 119.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 13.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 38.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 76.4,
            "unit": "µs",
            "extra": "min 71.8 µs, p95 87.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 104.1,
            "unit": "µs",
            "extra": "min 92.3 µs, p95 124.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 14,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 40.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 265.7,
            "unit": "µs",
            "extra": "min 225.5 µs, p95 383.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 364.2,
            "unit": "µs",
            "extra": "min 343.8 µs, p95 447.9 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 28509.3,
            "unit": "µs",
            "extra": "min 28087.2 µs, p95 29486.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 29810.9,
            "unit": "µs",
            "extra": "min 29354.3 µs, p95 30752.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 34756,
            "unit": "µs",
            "extra": "min 34429.5 µs, p95 36534 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 211.2,
            "unit": "µs",
            "extra": "min 202.7 µs, p95 227.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 260.9,
            "unit": "µs",
            "extra": "min 249.2 µs, p95 306.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 308.2,
            "unit": "µs",
            "extra": "min 294.6 µs, p95 327.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.61,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.83,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.25,
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
            "value": 0.129,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.143,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 5.21,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.74,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 21.15,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 21.64,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.29,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.31,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.41,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "feae5e0ac9aac40b1fdad50e377c734b6613a4b7",
          "message": "feat: S1.1 a state callback that fails once, and a fresh provider per verifier\n\nToday's verifier retries a state-change call that gets a 5xx, 408 or 429,\neight times with a back-off from 500 ms; janus verify's http hook does not\nretry. The corpus could not see the difference: its failing callback failed\nevery time. The scripted provider gains a `fail-once` callback (a 503, then\nsuccess), and corpora/verifier/state-callbacks/fails-once shows it: today\npasses, janus verify fails with hook-http-failed. Two ledger entries, under\nS1.9.\n\nIt first showed nothing, because both verifiers shared one provider per case\nand today's verifier used up the single failure. Each verifier now gets a\nfreshly started provider, so nothing one does to a provider changes what the\nother sees. No other verdict moved: 177 agree, 19 known.\n\nThe plan: S1.9 records the behaviour exactly (8 retries, those statuses, no\nCLI or FFI setting; an environment override, PACT_PROVIDER_STATE_CHANGE_RETRIES,\nproposed upstream) and the decision it leaves, the http hook's retry default.\nS1.13's table covers behaviour with no flag as well.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T13:33:13+11:00",
          "tree_id": "67f7e7d717792bd51b38048f0cbb4deb65605ec1",
          "url": "https://github.com/rholshausen/pact-janus/commit/feae5e0ac9aac40b1fdad50e377c734b6613a4b7"
        },
        "date": 1792291884631,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.8,
            "unit": "µs",
            "extra": "min 8.4 µs, p95 12 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2040.4,
            "unit": "µs",
            "extra": "min 1993.5 µs, p95 2138.5 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 204.2,
            "unit": "µs",
            "extra": "min 174.3 µs, p95 305.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2217.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6881899,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34152.1,
            "unit": "µs",
            "extra": "min 33545.9 µs, p95 34811.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36229.1,
            "unit": "µs",
            "extra": "min 35252.4 µs, p95 37107.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40705.4,
            "unit": "µs",
            "extra": "min 40222.8 µs, p95 41272.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 426.4,
            "unit": "µs",
            "extra": "min 408.2 µs, p95 449.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 521.5,
            "unit": "µs",
            "extra": "min 468.2 µs, p95 569.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 478.4,
            "unit": "µs",
            "extra": "min 455.9 µs, p95 500 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17171.9,
            "unit": "µs",
            "extra": "min 16111.3 µs, p95 18312.8 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18227.9,
            "unit": "µs",
            "extra": "min 17904.6 µs, p95 18465.4 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 18.9,
            "unit": "µs",
            "extra": "min 18.5 µs, p95 19.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.7,
            "unit": "µs",
            "extra": "min 25.3 µs, p95 26.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 25077.2,
            "unit": "µs",
            "extra": "min 23498.4 µs, p95 25943.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 25341.2,
            "unit": "µs",
            "extra": "min 23743.5 µs, p95 31221.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13924,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 14070.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 146.3,
            "unit": "µs",
            "extra": "min 134.7 µs, p95 165.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 176.8,
            "unit": "µs",
            "extra": "min 157.6 µs, p95 192.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 27.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 62.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 135.9,
            "unit": "µs",
            "extra": "min 120 µs, p95 150.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 172.7,
            "unit": "µs",
            "extra": "min 147.5 µs, p95 192.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 60.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 393.2,
            "unit": "µs",
            "extra": "min 297 µs, p95 572.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 504.4,
            "unit": "µs",
            "extra": "min 432.5 µs, p95 593.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33498.2,
            "unit": "µs",
            "extra": "min 33106.7 µs, p95 34023 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35158.2,
            "unit": "µs",
            "extra": "min 33949.8 µs, p95 36370.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40752.3,
            "unit": "µs",
            "extra": "min 40310.8 µs, p95 41496.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 279.7,
            "unit": "µs",
            "extra": "min 271.8 µs, p95 312.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 362.1,
            "unit": "µs",
            "extra": "min 348.9 µs, p95 382.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 397.5,
            "unit": "µs",
            "extra": "min 384.4 µs, p95 419.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.2,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.58,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.3,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.34,
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
            "value": 0.212,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.62,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.941,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 26.09,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.35,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.4,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.41,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.52,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.56,
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
          "id": "178cb64886787258736b3662c79af4fa503c5957",
          "message": "feat: S1.18 a regex in a v1-v4 pact matches the whole value\n\nToday's verifiers match a pact's regex against the whole value:\npact-reference with Oniguruma's is_match, pact-jvm with Kotlin's\nString.matches. Janus matched it anywhere, on the shape-language spec's\npremise that v1-v4 do, which S1.1's harness showed was wrong: \\d{4}-...Z\naccepted '2026-07-30T09:00:00Z-changed' and PENDING|SHIPPED accepted\n'NOT SHIPPED'.\n\n- The legacy compiler emits a pact's pattern anchored, ^(?:...)$, always,\n  since a pattern that looks anchored may not be (^a|b$ matches xb).\n  plan::legacy_anchored_pattern is the one place that says so.\n- janus upgrade writes the anchored pattern, so a converted pact keeps its\n  verdicts. Shapes are unchanged: a shape's regex matches anywhere.\n- Shape-language §4.2 no longer rests on v1-v4; plan-grammar §4.4 and\n  contract-file §8.2 say both readers anchor; the v3 upgrade example shows\n  the anchored output.\n- corpora: legacy/v3-regex-whole-value and legacy/upgrade-regex-anchored\n  (the pact and its upgraded contract refuse the same value); two snapshots\n  and one expected message now show the anchored pattern, no verdict moved.\n- The harness: the shippedAt disagreement is gone. The repeated-header one\n  (S3.3) moved from the negative case to the positive one, since the list\n  is now judged as a whole value; the ledger says so. 179 agree, 17 known.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T13:59:00+11:00",
          "tree_id": "0c406f77500f1c8030acc3fdee298428bfc8a533",
          "url": "https://github.com/rholshausen/pact-janus/commit/178cb64886787258736b3662c79af4fa503c5957"
        },
        "date": 1792293194964,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 13.8,
            "unit": "µs",
            "extra": "min 8.7 µs, p95 18.5 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2042.3,
            "unit": "µs",
            "extra": "min 1987 µs, p95 2231.8 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 195,
            "unit": "µs",
            "extra": "min 161.2 µs, p95 325.6 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2240.2,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6882041,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34238.7,
            "unit": "µs",
            "extra": "min 33549.3 µs, p95 35495.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36275.4,
            "unit": "µs",
            "extra": "min 35230.8 µs, p95 38383.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40862.5,
            "unit": "µs",
            "extra": "min 40036.9 µs, p95 41798 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 426.6,
            "unit": "µs",
            "extra": "min 412.1 µs, p95 446.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 520.4,
            "unit": "µs",
            "extra": "min 479.8 µs, p95 554.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 479.1,
            "unit": "µs",
            "extra": "min 453.9 µs, p95 502.9 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17315.2,
            "unit": "µs",
            "extra": "min 16534.3 µs, p95 19976.9 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18330.1,
            "unit": "µs",
            "extra": "min 18115.1 µs, p95 19091.9 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.9,
            "unit": "µs",
            "extra": "min 19.4 µs, p95 20.4 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.6,
            "unit": "µs",
            "extra": "min 25.3 µs, p95 26.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 23098.9,
            "unit": "µs",
            "extra": "min 22347 µs, p95 25714.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24426.5,
            "unit": "µs",
            "extra": "min 22540.6 µs, p95 30065.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12438.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13363.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 135.2,
            "unit": "µs",
            "extra": "min 130.8 µs, p95 146.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 170.2,
            "unit": "µs",
            "extra": "min 157.3 µs, p95 181.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 22.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 55.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 137.8,
            "unit": "µs",
            "extra": "min 120.8 µs, p95 147.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 172.5,
            "unit": "µs",
            "extra": "min 152 µs, p95 187.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 23.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 59.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 406,
            "unit": "µs",
            "extra": "min 279.7 µs, p95 512.6 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 495.3,
            "unit": "µs",
            "extra": "min 396.5 µs, p95 784.2 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33521.7,
            "unit": "µs",
            "extra": "min 32945.2 µs, p95 34496 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35258.8,
            "unit": "µs",
            "extra": "min 34222.4 µs, p95 37125.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40832.6,
            "unit": "µs",
            "extra": "min 40201.1 µs, p95 41551.2 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 281.8,
            "unit": "µs",
            "extra": "min 272.1 µs, p95 306.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 369.2,
            "unit": "µs",
            "extra": "min 354.6 µs, p95 396.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 400.4,
            "unit": "µs",
            "extra": "min 388.8 µs, p95 421.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.2,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.54,
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
            "value": 0.34,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.181,
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
            "value": 6.77,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.977,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.01,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 26.78,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.39,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "dbf92de824857fab54c2a01fd46f87a1d290befb",
          "message": "feat: S1.2 the broker client is the engine's: ADR 0032 and the protocol operations\n\nADR 0032 decides where the Pact Broker client lives. It is the engine's,\nbehind the protocol, so no host speaks the broker's API: a client in the CLI\nwould be rewritten by every SDK's provider integration (S1.15). It is a\nnative host capability, registered by janus and janus-engine as the hook\nimplementations are, so the kernel knows no HTTP and the offline WASM\nartifact has none. Verification fetches and publishes inside the run, so\npending status, publishing links and interaction ids never pass through a\nhost. Credentials travel in the request only; the engine never reads them\nfrom its environment and never repeats them.\n\nEngine-protocol spec:\n- §8.3: verification/verify takes a `broker` source (broker, provider,\n  consumer version selectors, provider branch and tags, pending, WIP) and a\n  `publish` option (each pact's result to its own link). A pending pact's\n  failures do not fail the run; a failed fetch ends the run with an error;\n  no pacts is pacts-not-found unless require-pacts is false. Replaces the\n  prototype's \"fetching from a broker is host business\".\n- §8.7: the broker descriptor (url, auth, S1.5's client options), the\n  credentials rule, and broker/publish-contracts in a broker session.\n- The `broker` capability, a fourth session kind, four event kinds, the\n  pacts-not-found code; broker.schema.json; the verification, events and\n  error schemas extended; examples/broker-verification.md, which the\n  example checker now validates.\n\nThe SDK bindings are regenerated: BrokerDescriptor, BrokerAuth,\nConsumerVersionSelector, PublishResults, VerifyOptions, PublishContracts.\n\nNo engine has a client until S1.3, so a broker source, a publish option and\nbroker/publish-contracts answer component-unavailable naming `broker`, as\nthe spec says; a broker source used to get operation-unsupported.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T14:48:16+11:00",
          "tree_id": "7d80a866be9113dbe6779c605c2bf5956e4d2d2d",
          "url": "https://github.com/rholshausen/pact-janus/commit/dbf92de824857fab54c2a01fd46f87a1d290befb"
        },
        "date": 1792295546922,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 7.7,
            "unit": "µs",
            "extra": "min 7.4 µs, p95 9.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1566.4,
            "unit": "µs",
            "extra": "min 1472.7 µs, p95 1741.8 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 112.4,
            "unit": "µs",
            "extra": "min 78.4 µs, p95 132 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1860.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6899613,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 28112,
            "unit": "µs",
            "extra": "min 27809.5 µs, p95 29527.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 29180.3,
            "unit": "µs",
            "extra": "min 28821.1 µs, p95 29511 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 30566,
            "unit": "µs",
            "extra": "min 30324.2 µs, p95 30762.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 391.7,
            "unit": "µs",
            "extra": "min 371 µs, p95 406.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 439.5,
            "unit": "µs",
            "extra": "min 399.1 µs, p95 482.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 407.3,
            "unit": "µs",
            "extra": "min 380.9 µs, p95 424.1 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 15248.1,
            "unit": "µs",
            "extra": "min 15074.6 µs, p95 16029.4 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 13656.8,
            "unit": "µs",
            "extra": "min 13488.9 µs, p95 13990.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 16.3,
            "unit": "µs",
            "extra": "min 15.7 µs, p95 16.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 19.2,
            "unit": "µs",
            "extra": "min 18.4 µs, p95 20.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22991.6,
            "unit": "µs",
            "extra": "min 20511.6 µs, p95 24822.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 23012.4,
            "unit": "µs",
            "extra": "min 21137 µs, p95 27969.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13491.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13283,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 98.6,
            "unit": "µs",
            "extra": "min 85.5 µs, p95 119.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 122.3,
            "unit": "µs",
            "extra": "min 101.1 µs, p95 143.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 27.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 51,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 90.4,
            "unit": "µs",
            "extra": "min 74.8 µs, p95 105.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 116.8,
            "unit": "µs",
            "extra": "min 91.7 µs, p95 150.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 18.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 48.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 262.9,
            "unit": "µs",
            "extra": "min 207.9 µs, p95 388.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 417.1,
            "unit": "µs",
            "extra": "min 383.2 µs, p95 489.7 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 27116.9,
            "unit": "µs",
            "extra": "min 26887.8 µs, p95 29232 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 28155.8,
            "unit": "µs",
            "extra": "min 27806.3 µs, p95 32300.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 30202.2,
            "unit": "µs",
            "extra": "min 29958.4 µs, p95 30841 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 249,
            "unit": "µs",
            "extra": "min 235.7 µs, p95 265.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 296.5,
            "unit": "µs",
            "extra": "min 282.8 µs, p95 319.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 325.1,
            "unit": "µs",
            "extra": "min 314.1 µs, p95 338.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.76,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.08,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.27,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.3,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.142,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.165,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 4.84,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.641,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 22.86,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 23.57,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.27,
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
            "value": 0.4,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.43,
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
          "id": "f419f4d6d927ba223e671e05aa93e098bb0e1f78",
          "message": "docs: S1.3 how the broker integration is tested\n\nNo one source covers it, so S1.3 now names four layers and why each:\na real, pinned broker in CI with today's verifier beside janus verify\n(the gate: fetched pacts, requests, published results and the matrix\ncompared, credentials checked absent); responses recorded from it for\noffline tests on all three systems; a consumer pact of the client's own,\nverified against that broker; and a schema lint against PactFlow's\nOpenAPI document, fetched from the SwaggerHub registry by a pinned\ndigest and never committed. It records what was looked at and not\nused, and the exit now includes the consumer pact and the lint.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T10:13:15+11:00",
          "tree_id": "2180ab03bd0f258671bdcd7941506d43266886f6",
          "url": "https://github.com/rholshausen/pact-janus/commit/f419f4d6d927ba223e671e05aa93e098bb0e1f78"
        },
        "date": 1792365494404,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10.9,
            "unit": "µs",
            "extra": "min 10.5 µs, p95 13.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2162.6,
            "unit": "µs",
            "extra": "min 2066.4 µs, p95 2601 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 228.7,
            "unit": "µs",
            "extra": "min 179.7 µs, p95 296.9 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2245.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6899613,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 35584.5,
            "unit": "µs",
            "extra": "min 34292.8 µs, p95 37175 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 37161,
            "unit": "µs",
            "extra": "min 35494.2 µs, p95 38741 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41140.4,
            "unit": "µs",
            "extra": "min 40629.2 µs, p95 41795.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 428.4,
            "unit": "µs",
            "extra": "min 409.1 µs, p95 452.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 542.1,
            "unit": "µs",
            "extra": "min 481.6 µs, p95 575.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 482.3,
            "unit": "µs",
            "extra": "min 459.5 µs, p95 512.4 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18588.1,
            "unit": "µs",
            "extra": "min 16177.6 µs, p95 20421.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19659.5,
            "unit": "µs",
            "extra": "min 18524.3 µs, p95 20314.7 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19,
            "unit": "µs",
            "extra": "min 18.4 µs, p95 19.4 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.5,
            "unit": "µs",
            "extra": "min 25.2 µs, p95 26 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 24171.9,
            "unit": "µs",
            "extra": "min 22206.8 µs, p95 25899.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 25828.5,
            "unit": "µs",
            "extra": "min 22999.4 µs, p95 29249.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13274.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 14410.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 151.4,
            "unit": "µs",
            "extra": "min 136.5 µs, p95 165.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 182.8,
            "unit": "µs",
            "extra": "min 161 µs, p95 212 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 28,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 63.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 138.3,
            "unit": "µs",
            "extra": "min 122.9 µs, p95 148.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 190.1,
            "unit": "µs",
            "extra": "min 152.2 µs, p95 205.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 23.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 68,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 400.9,
            "unit": "µs",
            "extra": "min 271.6 µs, p95 499.9 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 554.4,
            "unit": "µs",
            "extra": "min 444.2 µs, p95 995.3 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34523,
            "unit": "µs",
            "extra": "min 33274.5 µs, p95 35997 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35390.7,
            "unit": "µs",
            "extra": "min 34366.6 µs, p95 36782.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41379.2,
            "unit": "µs",
            "extra": "min 40756 µs, p95 41999 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 283.4,
            "unit": "µs",
            "extra": "min 272.8 µs, p95 311.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 420,
            "unit": "µs",
            "extra": "min 339.9 µs, p95 447.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 400.4,
            "unit": "µs",
            "extra": "min 388.1 µs, p95 423.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.26,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.63,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.32,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.36,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.19,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.218,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.98,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 1.023,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 27.84,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.55,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.4,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.41,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.55,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.57,
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
          "id": "a8bed6e20df8cbfa039f7bc8684d8d6566c47f87",
          "message": "feat: S1.3 the differential harness verifies from a pinned Pact Broker\n\nThe first step of S1.3, before any client code: S1.1's harness now runs\nbroker fixtures against a real broker, so \"correct\" is fixed before the\ncode that has to meet it.\n\n- tools/verifier-diff/pact-broker.json pins the Ruby broker image\n  (2.121.2) by digest; .github/scripts/pact-broker.sh starts it, the\n  harness refuses another version, and CI's verifier-diff job runs it.\n- A fixture's `broker` member publishes its pacts (version, branch,\n  tags), seeds earlier verification results, and gives both verifiers\n  the same selectors, pending, WIP and publish options. Each verifier's\n  pacticipants carry a prefix of their own, so neither sees what the\n  other did to the broker.\n- A broker case is also compared on what each run published per pact\n  (`<pact>/published`), and an interaction not fetched is `absent`.\n- A recording proxy in front of the broker keeps every request each\n  verifier makes, in the run's JSON; recorded, not yet compared.\n- Fixtures: the compatibility suite's four broker scenarios (no longer\n  skipped), the order-service pact, pending, not pending, WIP, not\n  selected, and pending with no selectors.\n- 35 ledger entries, all S1.3: janus verify is given today's flag\n  spellings, which it does not have yet.\n\nFound doing it: pact_verifier_cli 1.3.5 ignores --enable-pending when\nno selectors are given (it uses the broker's older /latest endpoint),\nwhich broker/pending-without-selectors pins; and it reports a pending\npact's failures under `pendingErrors`, which the harness now reads.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T10:55:59+11:00",
          "tree_id": "c856cd34d629dfa2da8221f2f34b50ab53712f97",
          "url": "https://github.com/rholshausen/pact-janus/commit/a8bed6e20df8cbfa039f7bc8684d8d6566c47f87"
        },
        "date": 1792368574560,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.6,
            "unit": "µs",
            "extra": "min 8.3 µs, p95 11.8 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2033,
            "unit": "µs",
            "extra": "min 1965.7 µs, p95 2141.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 190.3,
            "unit": "µs",
            "extra": "min 143 µs, p95 1076.5 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2220.7,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6899613,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34802.2,
            "unit": "µs",
            "extra": "min 33989.7 µs, p95 36099.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36481.1,
            "unit": "µs",
            "extra": "min 35347.4 µs, p95 37821.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41298.8,
            "unit": "µs",
            "extra": "min 40688.2 µs, p95 41718.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 432.2,
            "unit": "µs",
            "extra": "min 414.7 µs, p95 452.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 509.6,
            "unit": "µs",
            "extra": "min 472 µs, p95 553.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 485.4,
            "unit": "µs",
            "extra": "min 460.2 µs, p95 512.1 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17445.1,
            "unit": "µs",
            "extra": "min 16162.2 µs, p95 20051.5 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18897.5,
            "unit": "µs",
            "extra": "min 18320.4 µs, p95 19771.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.5,
            "unit": "µs",
            "extra": "min 19 µs, p95 19.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26.1,
            "unit": "µs",
            "extra": "min 25.7 µs, p95 26.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 24502.3,
            "unit": "µs",
            "extra": "min 22538 µs, p95 26318.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24234,
            "unit": "µs",
            "extra": "min 22678.4 µs, p95 29937.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13531,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13376.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 145,
            "unit": "µs",
            "extra": "min 124.1 µs, p95 151.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 172.1,
            "unit": "µs",
            "extra": "min 157.5 µs, p95 182.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 26.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 59.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 135.2,
            "unit": "µs",
            "extra": "min 117.9 µs, p95 145.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 171.1,
            "unit": "µs",
            "extra": "min 142.8 µs, p95 185.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 23.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 62.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 404.3,
            "unit": "µs",
            "extra": "min 335.1 µs, p95 603.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 481.4,
            "unit": "µs",
            "extra": "min 422 µs, p95 649.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34313.6,
            "unit": "µs",
            "extra": "min 33539.1 µs, p95 35310.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34951.3,
            "unit": "µs",
            "extra": "min 34129.4 µs, p95 36310 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41212.4,
            "unit": "µs",
            "extra": "min 40591.8 µs, p95 41777 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 282.6,
            "unit": "µs",
            "extra": "min 272.2 µs, p95 303.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 360.6,
            "unit": "µs",
            "extra": "min 349.2 µs, p95 381.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 398.7,
            "unit": "µs",
            "extra": "min 385.9 µs, p95 421.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.17,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.48,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.3,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.34,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.178,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.203,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.59,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.936,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.53,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 26.57,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.4,
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
            "value": 0.51,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "1f5fd9ca757a33e1a7e502c6231239c0cd498beb",
          "message": "feat: S1.3 janus verify fetches from a Pact Broker and publishes results\n\nADR 0032 built: the broker client is the engine's, behind the protocol,\nand a native host capability.\n\n- engine/kernel/src/broker.rs: the BrokerClient trait, in the broker's\n  terms (fetch the pacts for verification; publish a fetched pact's\n  result), and the broker descriptor, whose credentials never reach\n  Debug, an event, a summary or an error.\n- verification/verify: a `broker` source is fetched inside the run and\n  reported as verification/fetched; a failed fetch ends the run as\n  component-failed naming broker, an empty one as pacts-not-found\n  (unless require-pacts: false). options.publish posts each pact's\n  result to its own link, keyed by the broker's interaction ids (now in\n  each legacy interaction reference as `id`), as verification/published\n  events and the summary's `publication`.\n- Pending: a pending or WIP pact's failures, and a v4 interaction\n  marked pending from any source, are reported with `pending: true`,\n  counted, and do not fail the run. The summary says per source\n  whether it was pending.\n- engine/broker-client: the HAL client, asking as pact_verifier does;\n  its errors are broker-unreachable, broker-refused and\n  broker-response-invalid. Registered by both native binaries, so\n  engine/hello declares `broker`; the WASM artifact still has none.\n- janus verify: today's broker flags, in pact_verifier_cli's spellings,\n  with credentials from PACT_BROKER_USERNAME/PASSWORD/TOKEN. Pending and\n  WIP are asked for only with selectors, as today's verifier does.\n- broker/publish-contracts is not built yet: an engine with a client\n  answers operation-unsupported.\n\nThe differential harness now agrees with today's verifier on every\nbroker case, published results included, and on the suite's pending v4\ninteraction: all 36 S1.3 ledger entries are deleted. The broker traffic\nof the two matches request for request but for today's extra GET of\nthe for-verification link.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T11:45:55+11:00",
          "tree_id": "45f3f1e5195dfe6170b0c6f4d01031b3bf5b793b",
          "url": "https://github.com/rholshausen/pact-janus/commit/1f5fd9ca757a33e1a7e502c6231239c0cd498beb"
        },
        "date": 1792371027947,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.8,
            "unit": "µs",
            "extra": "min 8.5 µs, p95 38 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2068.7,
            "unit": "µs",
            "extra": "min 1995.4 µs, p95 2164.2 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 206.2,
            "unit": "µs",
            "extra": "min 158.2 µs, p95 974 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2271.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6975491,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34115.4,
            "unit": "µs",
            "extra": "min 33500.8 µs, p95 34841.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35039.8,
            "unit": "µs",
            "extra": "min 34414.1 µs, p95 35863.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40723.1,
            "unit": "µs",
            "extra": "min 40309.5 µs, p95 41224.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 427.6,
            "unit": "µs",
            "extra": "min 410.3 µs, p95 458 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 505.3,
            "unit": "µs",
            "extra": "min 460.7 µs, p95 548.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 482.5,
            "unit": "µs",
            "extra": "min 459.9 µs, p95 506.3 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16345.5,
            "unit": "µs",
            "extra": "min 15788.6 µs, p95 17289.3 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18593.6,
            "unit": "µs",
            "extra": "min 18336.3 µs, p95 19062.6 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.1,
            "unit": "µs",
            "extra": "min 18.7 µs, p95 19.5 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.7,
            "unit": "µs",
            "extra": "min 25.3 µs, p95 26.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 23437.9,
            "unit": "µs",
            "extra": "min 22153.2 µs, p95 24868.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 23322.8,
            "unit": "µs",
            "extra": "min 21861.9 µs, p95 29362.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12806.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12692.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 145.4,
            "unit": "µs",
            "extra": "min 125.7 µs, p95 151.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 171.7,
            "unit": "µs",
            "extra": "min 158.7 µs, p95 183.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 27,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 56.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 135.7,
            "unit": "µs",
            "extra": "min 122.8 µs, p95 147.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 172.2,
            "unit": "µs",
            "extra": "min 148.3 µs, p95 188.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 59.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 389.7,
            "unit": "µs",
            "extra": "min 243.2 µs, p95 827 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 496.5,
            "unit": "µs",
            "extra": "min 442.3 µs, p95 722.2 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33806.2,
            "unit": "µs",
            "extra": "min 33363.8 µs, p95 34428 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34035.9,
            "unit": "µs",
            "extra": "min 33435.4 µs, p95 34530.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41032,
            "unit": "µs",
            "extra": "min 40594 µs, p95 41479.7 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 278.6,
            "unit": "µs",
            "extra": "min 269.5 µs, p95 303.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 358.4,
            "unit": "µs",
            "extra": "min 328.7 µs, p95 384 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 397,
            "unit": "µs",
            "extra": "min 386.2 µs, p95 419.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.17,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.48,
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
            "value": 0.34,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.181,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.2,
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
            "value": 0.91,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.11,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.51,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.39,
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
            "value": 0.52,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.56,
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
          "id": "83267ed5cf373ff3e08f1f02dcfcb58af57648a9",
          "message": "fix: write the CLI tests' shared consumer contract once, not once per test\n\nSix tests in cli/tests/cli.rs each rewrote the same temporary contract file,\nand they run in parallel. `std::fs::write` truncates before it writes, so a\n`janus check` one test had started could read the file empty — \"EOF while\nparsing a value at line 1 column 0\", seen on CI's Linux runner. Latent since\ntask 7.4; the file is now written once per test process.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-16T15:40:41+11:00",
          "tree_id": "e98e661825d4ff0b7e0bff9eaf0864b496eb8f5e",
          "url": "https://github.com/rholshausen/pact-janus/commit/83267ed5cf373ff3e08f1f02dcfcb58af57648a9"
        },
        "date": 1792126440340,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28643,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21731,
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
          "id": "741588470b75856ddb5cc5824db77823d28e8c29",
          "message": "feat: S0.11 the SDK gates and the executable specification ship with the engine\n\nEvery release now carries janus-sdk-gates-<triple> for the five triples and\njanus-spec.tar.gz (ADR 0030, extending ADR 0028 decision 9): 17 files. A\ncheckout holding one SDK directory runs its conformance, thinness and\nbindings gates from those files alone, which release.yml's new sdk-gates job\nproves on every rehearsal and before every publish.\n\n- tools/conformance, tools/thinness and tools/bindings move their commands\n  into their libraries and take --spec <dir> (a checkout or an unpacked\n  janus-spec.tar.gz) in place of the source path compiled into them;\n  tools/sdk-gates is the three as one binary.\n- janus-spec.tar.gz is the corpora, the conformance suite, the behavioural\n  specification, sdks/bindings.json and its schema sets, at their paths here,\n  made by `git archive` (.github/scripts/janus-spec.sh).\n- Each SDK keeps its own thinness.json, with paths relative to it; with none\n  named, the audit measures every sdks/*/thinness.json side by side.\n- Both SDKs' drivers read JANUS_SPEC for the conformance cases and schemas.\n- The Change file workflow counts the gates' sources and the specification's\n  files as shipped.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-16T16:31:07+11:00",
          "tree_id": "79355d29920c7010cd839b6f8d89e7be7af684b7",
          "url": "https://github.com/rholshausen/pact-janus/commit/741588470b75856ddb5cc5824db77823d28e8c29"
        },
        "date": 1792129485364,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 29501,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 22056,
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
          "id": "7ffba9f5f29cbd8cda33e67c24d3c6dca106fd11",
          "message": "feat: S0.5 rename-ready under the codename\n\nThe codename is renamed by one command, `.github/scripts/rename.sh <name>`,\nand the `Rename rehearsal` workflow runs it every week on a throwaway\ncheckout (ADR 0031; the inventory is Documentation/codename.md).\n\n- The script rewrites janus/Janus/JANUS in every text file, the record\n  included, moves every path that contains it, and fails unless nothing is\n  left but CHANGELOG.md, the earlier spellings and the repository's own name\n  (S0.7's). It refuses a name that is not one lowercase word, or that the\n  repository already contains.\n- Persisted identifiers, names the engine reads back from documents that\n  outlive it, are engine/kernel/src/persisted.rs: each written under one\n  spelling and read under every earlier one, which the script records in\n  persisted/earlier.rs before it rewrites. Seven of them: a contract's and a\n  provider shape's $format, the OCI component artifact and config types, the\n  interface a component exports, the hook script global and the http hook's\n  body format. Their readers now call reads().\n- The rehearsal builds a component before renaming and loads it after, and\n  requires clippy, every test, both SDK suites and their conformance,\n  thinness, the binding pipeline, the protocol client and the demo to pass,\n  renamed. kernel tests/persisted.rs reads documents under each earlier\n  spelling, which only a rename provides.\n\nStage 0a is done.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-16T18:51:34+11:00",
          "tree_id": "5d7c4d7cdeeca76476415849e057bc5eee0828cc",
          "url": "https://github.com/rholshausen/pact-janus/commit/7ffba9f5f29cbd8cda33e67c24d3c6dca106fd11"
        },
        "date": 1792137949624,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 27144,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21512,
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
          "id": "e6fff87c364b45e453fde7206dbddc896bfd4653",
          "message": "feat: the janus CLI rebuilt on clap, ready for stage 1a\n\nThe CLI was built for the prototype, with a hand-rolled argument parser.\nBefore stage 1a puts it in front of providers, it is a proper command line:\none clap declaration (cli/src/args.rs) is the flags, the help, the usage\nerrors and their exit code.\n\nBreaking, with a change file (major): --json is now --output text|json on\nverify, check, upgrade and component push/pull, the spelling the Pact Broker\nclient uses, leaving --json <file> free for pact_verifier_cli parity (S1.13).\nupgrade --out is now --write, so it cannot be confused with --output.\n\nFixed:\n- janus verify gave up when a poll waited 10s with no events, so a slow\n  provider or state hook failed a run that was still going. It now waits for\n  the terminal event. That needed a kernel fix first: a verification run that\n  panicked never ended its stream, and the 10s cut-off was hiding a hang. A\n  run now always ends with verification/finished, status failed and an\n  internal error (engine-protocol spec §9.6; its own change file, patch).\n- A closed stdout (janus ... | head) panicked in println!. Output goes\n  through outln!/errln!, which drop writes to a gone stream; the command runs\n  to its end and keeps its exit code.\n- Mistakes clap now refuses that were ignored: extra arguments, values that\n  are not warn|block, an --as-of that is not a calendar date, a non-numeric\n  --index, and flags that cannot be combined.\n- check --output json dropped the \"no provider shape\" warning on stderr.\n- janus-engine --version waited on stdin; it now answers --version and\n  --help. A Content-Length larger than the body sent is an EOF error, not an\n  allocation of the claimed size.\n\nAlso: one error type instead of ~30 hand-formatted failures; the commands\nin a library target, so both binaries share it without #[path]; help text\nwritten for users; --log-level, janus --version, janus completions <shell>;\ncoloured logs only on a terminal without NO_COLOR; the duplicated directory\nwalk in io.rs folded into one.\n\nNew dependencies: clap and clap_complete; trycmd and snapbox for tests.\nNone of rstest, expectest or pretty_assertions snapshots a command, so\nCLAUDE.md now says how these are used: cli/tests/cmd/ holds trycmd cases\n(every --help in help.md, the usage errors, offline commands), and\ncli/tests/cli.rs compares whole outputs with snapbox where a provider or\nregistry must run first. Engine-owned text is elided; corpora/ pins it.\n\nThe filtered-run snapshot shows a gap left alone here: a --variant that\nmatches nothing reports VERIFIED and exits 0. Staged plan S1.17 records it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T11:11:25+11:00",
          "tree_id": "8066214780a5c6953f5361cbb81b47641d4b5081",
          "url": "https://github.com/rholshausen/pact-janus/commit/e6fff87c364b45e453fde7206dbddc896bfd4653"
        },
        "date": 1792282745927,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28408,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21397,
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
          "id": "1ee0beb8cb2b7de74054e225980ddf811b0b4626",
          "message": "fix: help and usage errors name the program janus on Windows too\n\nclap takes the program's name from argv[0], which on Windows is janus.exe,\nso every Usage: line said `janus.exe ...` there and the trycmd snapshots of\nhelp and usage errors failed on the windows-latest job. The name is now\npinned with bin_name, in janus and in janus-engine. Reproduced and checked\non Linux by running copies of both binaries named *.exe.\n\nThe unreleased CLI change file gains a line for it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T11:35:02+11:00",
          "tree_id": "f7a739fabdef00de748d13649b746091cde05323",
          "url": "https://github.com/rholshausen/pact-janus/commit/1ee0beb8cb2b7de74054e225980ddf811b0b4626"
        },
        "date": 1792284240994,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 43390,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 35775,
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
          "id": "30e7234152d51261dc18c31daa03274e24f978f9",
          "message": "feat: S1.1 the differential verification harness and its fixture corpus\n\n\"Exactly as today\" is now a number. tools/verifier-diff runs today's\nverifier (pact_verifier_cli, pinned at 1.3.5 in\ntools/verifier-diff/pact-verifier.json) and janus verify on every pact in\ncorpora/verifier/ against the same provider, and compares the verdicts per\ninteraction and for the whole run. CI's new verifier-diff job runs it,\ninstalling the pinned verifier with .github/scripts/pact-verifier.sh, which\nrefuses any other digest.\n\n- The corpus: the order-service's v3 pact against the order-service itself;\n  the pact-compatibility-suite's HTTP provider scenarios V1-V4 (Apache-2.0,\n  commit 1acfa1e), each pact built with pact_models as the suite's own\n  harness builds it, 29 imported and 5 skipped with the plan item that lets\n  them in (S1.3 broker, S1.9 request filter); and deterministic mutations of\n  each interaction's response. 96 cases, 194 verdicts, in about two minutes.\n- The scripted provider recognises requests strictly by example, as pact's\n  mock server does, answers a fixture's routes, and takes provider states.\n- The ledger, corpora/verifier/disagreements.json, lists every disagreement\n  with the plan item or ADR that accounts for it. The run fails on one it\n  does not list, and on a listed one that has gone. Today: 177 agree, 17\n  known, nothing unexpected.\n\nWhat the first run found, in the plan:\n- S1.18 (new, decided): today's verifiers match a regex against the whole\n  value (Oniguruma's is_match in pact-reference, Kotlin's String.matches in\n  pact-jvm); the shape-language spec says v1-v4 match anywhere. The legacy\n  compiler and upgrade will anchor; shapes keep matching anywhere.\n- S1.6 gains plain-text bodies, which are component-unavailable today.\n- S1.9 notes that today's verifier retries a failing state call for about\n  two minutes and the http hook does not retry.\n- S3.3 keeps list-valued headers in stage 3, deliberately, with why.\n\njanus verify's summary lists failures but not passes, so passes are inferred\nfrom its counts, cross-checked; S1.3 needs per-variant results in it anyway.\ncargo test checks that every fixture and ledger entry reads, without a\nverifier installed. corpora/ ships in janus-spec.tar.gz, hence the change\nfile.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T12:44:44+11:00",
          "tree_id": "5c60919078d362a66bc68d36f1747a6f534fd429",
          "url": "https://github.com/rholshausen/pact-janus/commit/30e7234152d51261dc18c31daa03274e24f978f9"
        },
        "date": 1792288590664,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28289,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21709,
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
          "id": "60d1a8305581c72dc809869482a2207a01f4e6f5",
          "message": "docs: one set of agent instructions, in AGENTS.md\n\nCLAUDE.md and .github/copilot-instructions.md said the same things twice and\nhad drifted apart in structure, so every change was two edits that did not\nalways apply. AGENTS.md is now the only copy: Claude Code reads it through\nCLAUDE.md, which is just `@AGENTS.md`, and GitHub Copilot (the coding agent\nand PR review) and other agents read it directly.\n\n- CLAUDE.md moves to AGENTS.md, history kept. Its opening says who reads it.\n- .github/copilot-instructions.md is deleted. Compared sentence by sentence,\n  it held nothing CLAUDE.md lacked, and one stale line (`pact.execute`).\n- CONTRIBUTING.md and the live comments and specs that cite the rules now\n  cite AGENTS.md. The prototype record, the ADRs and the finished S0.5 entry\n  are left as written; CLAUDE.md still resolves for them.\n- The status paragraph records S1.1 done: its CI job passed on its first\n  run, with every disagreement in the ledger.\n\nNo-Changeset: comments and documentation only; nothing a release ships changes behaviour.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T13:01:35+11:00",
          "tree_id": "bb3ffcc3d68d4f6130340d3737e31eac18f6d60b",
          "url": "https://github.com/rholshausen/pact-janus/commit/60d1a8305581c72dc809869482a2207a01f4e6f5"
        },
        "date": 1792289135953,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 41128,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 29222,
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
          "id": "feae5e0ac9aac40b1fdad50e377c734b6613a4b7",
          "message": "feat: S1.1 a state callback that fails once, and a fresh provider per verifier\n\nToday's verifier retries a state-change call that gets a 5xx, 408 or 429,\neight times with a back-off from 500 ms; janus verify's http hook does not\nretry. The corpus could not see the difference: its failing callback failed\nevery time. The scripted provider gains a `fail-once` callback (a 503, then\nsuccess), and corpora/verifier/state-callbacks/fails-once shows it: today\npasses, janus verify fails with hook-http-failed. Two ledger entries, under\nS1.9.\n\nIt first showed nothing, because both verifiers shared one provider per case\nand today's verifier used up the single failure. Each verifier now gets a\nfreshly started provider, so nothing one does to a provider changes what the\nother sees. No other verdict moved: 177 agree, 19 known.\n\nThe plan: S1.9 records the behaviour exactly (8 retries, those statuses, no\nCLI or FFI setting; an environment override, PACT_PROVIDER_STATE_CHANGE_RETRIES,\nproposed upstream) and the decision it leaves, the http hook's retry default.\nS1.13's table covers behaviour with no flag as well.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T13:33:13+11:00",
          "tree_id": "67f7e7d717792bd51b38048f0cbb4deb65605ec1",
          "url": "https://github.com/rholshausen/pact-janus/commit/feae5e0ac9aac40b1fdad50e377c734b6613a4b7"
        },
        "date": 1792291886252,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 27880,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21096,
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
          "id": "178cb64886787258736b3662c79af4fa503c5957",
          "message": "feat: S1.18 a regex in a v1-v4 pact matches the whole value\n\nToday's verifiers match a pact's regex against the whole value:\npact-reference with Oniguruma's is_match, pact-jvm with Kotlin's\nString.matches. Janus matched it anywhere, on the shape-language spec's\npremise that v1-v4 do, which S1.1's harness showed was wrong: \\d{4}-...Z\naccepted '2026-07-30T09:00:00Z-changed' and PENDING|SHIPPED accepted\n'NOT SHIPPED'.\n\n- The legacy compiler emits a pact's pattern anchored, ^(?:...)$, always,\n  since a pattern that looks anchored may not be (^a|b$ matches xb).\n  plan::legacy_anchored_pattern is the one place that says so.\n- janus upgrade writes the anchored pattern, so a converted pact keeps its\n  verdicts. Shapes are unchanged: a shape's regex matches anywhere.\n- Shape-language §4.2 no longer rests on v1-v4; plan-grammar §4.4 and\n  contract-file §8.2 say both readers anchor; the v3 upgrade example shows\n  the anchored output.\n- corpora: legacy/v3-regex-whole-value and legacy/upgrade-regex-anchored\n  (the pact and its upgraded contract refuse the same value); two snapshots\n  and one expected message now show the anchored pattern, no verdict moved.\n- The harness: the shippedAt disagreement is gone. The repeated-header one\n  (S3.3) moved from the negative case to the positive one, since the list\n  is now judged as a whole value; the ledger says so. 179 agree, 17 known.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T13:59:00+11:00",
          "tree_id": "0c406f77500f1c8030acc3fdee298428bfc8a533",
          "url": "https://github.com/rholshausen/pact-janus/commit/178cb64886787258736b3662c79af4fa503c5957"
        },
        "date": 1792293197086,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28888,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21228,
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
          "id": "dbf92de824857fab54c2a01fd46f87a1d290befb",
          "message": "feat: S1.2 the broker client is the engine's: ADR 0032 and the protocol operations\n\nADR 0032 decides where the Pact Broker client lives. It is the engine's,\nbehind the protocol, so no host speaks the broker's API: a client in the CLI\nwould be rewritten by every SDK's provider integration (S1.15). It is a\nnative host capability, registered by janus and janus-engine as the hook\nimplementations are, so the kernel knows no HTTP and the offline WASM\nartifact has none. Verification fetches and publishes inside the run, so\npending status, publishing links and interaction ids never pass through a\nhost. Credentials travel in the request only; the engine never reads them\nfrom its environment and never repeats them.\n\nEngine-protocol spec:\n- §8.3: verification/verify takes a `broker` source (broker, provider,\n  consumer version selectors, provider branch and tags, pending, WIP) and a\n  `publish` option (each pact's result to its own link). A pending pact's\n  failures do not fail the run; a failed fetch ends the run with an error;\n  no pacts is pacts-not-found unless require-pacts is false. Replaces the\n  prototype's \"fetching from a broker is host business\".\n- §8.7: the broker descriptor (url, auth, S1.5's client options), the\n  credentials rule, and broker/publish-contracts in a broker session.\n- The `broker` capability, a fourth session kind, four event kinds, the\n  pacts-not-found code; broker.schema.json; the verification, events and\n  error schemas extended; examples/broker-verification.md, which the\n  example checker now validates.\n\nThe SDK bindings are regenerated: BrokerDescriptor, BrokerAuth,\nConsumerVersionSelector, PublishResults, VerifyOptions, PublishContracts.\n\nNo engine has a client until S1.3, so a broker source, a publish option and\nbroker/publish-contracts answer component-unavailable naming `broker`, as\nthe spec says; a broker source used to get operation-unsupported.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-18T14:48:16+11:00",
          "tree_id": "7d80a866be9113dbe6779c605c2bf5956e4d2d2d",
          "url": "https://github.com/rholshausen/pact-janus/commit/dbf92de824857fab54c2a01fd46f87a1d290befb"
        },
        "date": 1792295548920,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 49253,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 42168,
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
          "id": "f419f4d6d927ba223e671e05aa93e098bb0e1f78",
          "message": "docs: S1.3 how the broker integration is tested\n\nNo one source covers it, so S1.3 now names four layers and why each:\na real, pinned broker in CI with today's verifier beside janus verify\n(the gate: fetched pacts, requests, published results and the matrix\ncompared, credentials checked absent); responses recorded from it for\noffline tests on all three systems; a consumer pact of the client's own,\nverified against that broker; and a schema lint against PactFlow's\nOpenAPI document, fetched from the SwaggerHub registry by a pinned\ndigest and never committed. It records what was looked at and not\nused, and the exit now includes the consumer pact and the lint.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T10:13:15+11:00",
          "tree_id": "2180ab03bd0f258671bdcd7941506d43266886f6",
          "url": "https://github.com/rholshausen/pact-janus/commit/f419f4d6d927ba223e671e05aa93e098bb0e1f78"
        },
        "date": 1792365496738,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28251,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21771,
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
          "id": "a8bed6e20df8cbfa039f7bc8684d8d6566c47f87",
          "message": "feat: S1.3 the differential harness verifies from a pinned Pact Broker\n\nThe first step of S1.3, before any client code: S1.1's harness now runs\nbroker fixtures against a real broker, so \"correct\" is fixed before the\ncode that has to meet it.\n\n- tools/verifier-diff/pact-broker.json pins the Ruby broker image\n  (2.121.2) by digest; .github/scripts/pact-broker.sh starts it, the\n  harness refuses another version, and CI's verifier-diff job runs it.\n- A fixture's `broker` member publishes its pacts (version, branch,\n  tags), seeds earlier verification results, and gives both verifiers\n  the same selectors, pending, WIP and publish options. Each verifier's\n  pacticipants carry a prefix of their own, so neither sees what the\n  other did to the broker.\n- A broker case is also compared on what each run published per pact\n  (`<pact>/published`), and an interaction not fetched is `absent`.\n- A recording proxy in front of the broker keeps every request each\n  verifier makes, in the run's JSON; recorded, not yet compared.\n- Fixtures: the compatibility suite's four broker scenarios (no longer\n  skipped), the order-service pact, pending, not pending, WIP, not\n  selected, and pending with no selectors.\n- 35 ledger entries, all S1.3: janus verify is given today's flag\n  spellings, which it does not have yet.\n\nFound doing it: pact_verifier_cli 1.3.5 ignores --enable-pending when\nno selectors are given (it uses the broker's older /latest endpoint),\nwhich broker/pending-without-selectors pins; and it reports a pending\npact's failures under `pendingErrors`, which the harness now reads.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T10:55:59+11:00",
          "tree_id": "c856cd34d629dfa2da8221f2f34b50ab53712f97",
          "url": "https://github.com/rholshausen/pact-janus/commit/a8bed6e20df8cbfa039f7bc8684d8d6566c47f87"
        },
        "date": 1792368576658,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28993,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21920,
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
          "id": "1f5fd9ca757a33e1a7e502c6231239c0cd498beb",
          "message": "feat: S1.3 janus verify fetches from a Pact Broker and publishes results\n\nADR 0032 built: the broker client is the engine's, behind the protocol,\nand a native host capability.\n\n- engine/kernel/src/broker.rs: the BrokerClient trait, in the broker's\n  terms (fetch the pacts for verification; publish a fetched pact's\n  result), and the broker descriptor, whose credentials never reach\n  Debug, an event, a summary or an error.\n- verification/verify: a `broker` source is fetched inside the run and\n  reported as verification/fetched; a failed fetch ends the run as\n  component-failed naming broker, an empty one as pacts-not-found\n  (unless require-pacts: false). options.publish posts each pact's\n  result to its own link, keyed by the broker's interaction ids (now in\n  each legacy interaction reference as `id`), as verification/published\n  events and the summary's `publication`.\n- Pending: a pending or WIP pact's failures, and a v4 interaction\n  marked pending from any source, are reported with `pending: true`,\n  counted, and do not fail the run. The summary says per source\n  whether it was pending.\n- engine/broker-client: the HAL client, asking as pact_verifier does;\n  its errors are broker-unreachable, broker-refused and\n  broker-response-invalid. Registered by both native binaries, so\n  engine/hello declares `broker`; the WASM artifact still has none.\n- janus verify: today's broker flags, in pact_verifier_cli's spellings,\n  with credentials from PACT_BROKER_USERNAME/PASSWORD/TOKEN. Pending and\n  WIP are asked for only with selectors, as today's verifier does.\n- broker/publish-contracts is not built yet: an engine with a client\n  answers operation-unsupported.\n\nThe differential harness now agrees with today's verifier on every\nbroker case, published results included, and on the suite's pending v4\ninteraction: all 36 S1.3 ledger entries are deleted. The broker traffic\nof the two matches request for request but for today's extra GET of\nthe for-verification link.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T11:45:55+11:00",
          "tree_id": "45f3f1e5195dfe6170b0c6f4d01031b3bf5b793b",
          "url": "https://github.com/rholshausen/pact-janus/commit/1f5fd9ca757a33e1a7e502c6231239c0cd498beb"
        },
        "date": 1792371030338,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28574,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21832,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      }
    ]
  }
}