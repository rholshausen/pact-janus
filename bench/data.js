window.BENCHMARK_DATA = {
  "lastUpdate": 1792735225975,
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
          "id": "3c0bd6b06f548b4672c83173843ecaab938d049a",
          "message": "feat: S1.3 a run's summary counts its variants per source document\n\nPhase 9 finding 10, option A, decided in ADR 0033 and built.\n\n- The verification summary gains `pairs`, positionally parallel to\n  `consumers`/`providers`: per source document its consumer, provider,\n  status (verified, failed, incomplete) and its own variant counts,\n  which the run-level `variants` adds up. A pending document that\n  failed is `failed` there; the run's `status` is what forgives it.\n- Per source document rather than per (consumer, provider): a broker\n  run's main and deployed pacts for one consumer are different matrix\n  rows, and a sum would hide which one failed.\n- subsumption/decide reads the entries: a pair's counts are its\n  entries' added up, present whenever one run covered it, so janus\n  check reports counts for multi-contract runs. A summary from before\n  `pairs` is read as it was.\n- ADR 0033 indexed and dropped from the backlog; spec §9.6, the broker\n  worked example and subsumption.schema.json's descriptions updated,\n  and the bindings regenerated (doc comments only).\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T12:07:11+11:00",
          "tree_id": "0ba2d99b7e2a8cdf9b36e9b67f0c58bb3458c3a0",
          "url": "https://github.com/rholshausen/pact-janus/commit/3c0bd6b06f548b4672c83173843ecaab938d049a"
        },
        "date": 1792372303238,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.7,
            "unit": "µs",
            "extra": "min 8.1 µs, p95 16.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1619.6,
            "unit": "µs",
            "extra": "min 1553.1 µs, p95 1912.1 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 141.3,
            "unit": "µs",
            "extra": "min 118.2 µs, p95 280.3 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2242.6,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 6976055,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 35488.6,
            "unit": "µs",
            "extra": "min 33836.2 µs, p95 36604.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36495.8,
            "unit": "µs",
            "extra": "min 35031.6 µs, p95 37713.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 43179.9,
            "unit": "µs",
            "extra": "min 42055.7 µs, p95 44089.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 444.9,
            "unit": "µs",
            "extra": "min 426.3 µs, p95 477 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 504.1,
            "unit": "µs",
            "extra": "min 451.7 µs, p95 546 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 503.9,
            "unit": "µs",
            "extra": "min 481.5 µs, p95 520.5 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17684.1,
            "unit": "µs",
            "extra": "min 16814.7 µs, p95 19025.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18409.3,
            "unit": "µs",
            "extra": "min 17965.6 µs, p95 19090.8 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.6,
            "unit": "µs",
            "extra": "min 19.1 µs, p95 19.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.9,
            "unit": "µs",
            "extra": "min 25.1 µs, p95 26.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 27889.8,
            "unit": "µs",
            "extra": "min 25464.3 µs, p95 29888.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 27712.2,
            "unit": "µs",
            "extra": "min 25726.4 µs, p95 30429.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 16428,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 16089,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 101.4,
            "unit": "µs",
            "extra": "min 92.8 µs, p95 113.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 129.1,
            "unit": "µs",
            "extra": "min 114.3 µs, p95 146.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 21,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 47.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 93.3,
            "unit": "µs",
            "extra": "min 83.9 µs, p95 110.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 119.8,
            "unit": "µs",
            "extra": "min 108.2 µs, p95 135.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 18.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 45.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 284.6,
            "unit": "µs",
            "extra": "min 222.4 µs, p95 486.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 398.7,
            "unit": "µs",
            "extra": "min 340.9 µs, p95 466.2 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34414.5,
            "unit": "µs",
            "extra": "min 32999.1 µs, p95 47695.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35235.6,
            "unit": "µs",
            "extra": "min 33682.9 µs, p95 36516 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 44248.4,
            "unit": "µs",
            "extra": "min 43154 µs, p95 44963.8 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 292.6,
            "unit": "µs",
            "extra": "min 279 µs, p95 308.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 340.1,
            "unit": "µs",
            "extra": "min 319.4 µs, p95 361.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 399.8,
            "unit": "µs",
            "extra": "min 386.1 µs, p95 411.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.93,
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
            "value": 0.29,
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
            "value": 0.159,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.177,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 5.39,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.741,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 27.66,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.11,
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
            "value": 0.32,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.47,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.48,
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
          "id": "5d7f8a00cd5d63752deaadc73ec413cffb0e72d2",
          "message": "feat: S1.3 janus publish publishes a consumer's contracts to a Pact Broker\n\nbroker/publish-contracts is built (engine-protocol spec §8.7, ADR 0032): the kernel's broker\ntrait gains publish_contracts, the native client posts every contract to the index's\npb:publish-contracts link as the broker's 'pact' specification, and the engine reports the\noutcome in a broker session: one broker/published per contract, then broker/finished with the\nbroker's notices. Documents are read as verify reads them before anything is published, and are\npublished as sent.\n\njanus publish drives it with pact-broker publish's spellings and environment variables.\n\nCompression: the pinned broker (2.121.2) refuses a gzipped body with a 400 and advertises no\nAccept-Encoding, so the client gzips only when a broker says it can (RFC 7694) and resends\nuncompressed after a 415. Recorded in S1.3, the protocol spec and the contract-file spec.\n\nA live test publishes a pact and its upgraded Janus contract to the pinned broker and verifies\nboth back from it; CI's verifier-diff job runs it with JANUS_PACT_BROKER.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T14:21:37+11:00",
          "tree_id": "d04eff1e5ca901dbca9b87aff04feb969ed7b7a9",
          "url": "https://github.com/rholshausen/pact-janus/commit/5d7f8a00cd5d63752deaadc73ec413cffb0e72d2"
        },
        "date": 1792380352660,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10.9,
            "unit": "µs",
            "extra": "min 10.3 µs, p95 16 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2067.3,
            "unit": "µs",
            "extra": "min 2004.5 µs, p95 2103.3 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 174.5,
            "unit": "µs",
            "extra": "min 150.8 µs, p95 1095.7 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2270.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7107621,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33553.3,
            "unit": "µs",
            "extra": "min 33194.7 µs, p95 34112.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 34920,
            "unit": "µs",
            "extra": "min 34255.9 µs, p95 35662.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40515.9,
            "unit": "µs",
            "extra": "min 40147.2 µs, p95 40995.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 422.9,
            "unit": "µs",
            "extra": "min 409 µs, p95 440.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 520.6,
            "unit": "µs",
            "extra": "min 461.6 µs, p95 562.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 484.1,
            "unit": "µs",
            "extra": "min 459.8 µs, p95 505 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16159.9,
            "unit": "µs",
            "extra": "min 15766.5 µs, p95 16445.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18836.7,
            "unit": "µs",
            "extra": "min 18436.5 µs, p95 19043.8 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.2,
            "unit": "µs",
            "extra": "min 18.6 µs, p95 19.7 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.9,
            "unit": "µs",
            "extra": "min 25.6 µs, p95 26.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22473.6,
            "unit": "µs",
            "extra": "min 21523.9 µs, p95 24176.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 22736.7,
            "unit": "µs",
            "extra": "min 21849 µs, p95 25704.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12037.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12239.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 133.6,
            "unit": "µs",
            "extra": "min 129.4 µs, p95 148.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 180.9,
            "unit": "µs",
            "extra": "min 153.3 µs, p95 188.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 20.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 67.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 134.2,
            "unit": "µs",
            "extra": "min 116.1 µs, p95 144.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 176.4,
            "unit": "µs",
            "extra": "min 149.5 µs, p95 189.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 67.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 380.4,
            "unit": "µs",
            "extra": "min 316.2 µs, p95 479.7 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 485,
            "unit": "µs",
            "extra": "min 394.4 µs, p95 660.9 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 32729.2,
            "unit": "µs",
            "extra": "min 32378 µs, p95 33300.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 33893.2,
            "unit": "µs",
            "extra": "min 33336.6 µs, p95 34579.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40706.9,
            "unit": "µs",
            "extra": "min 40414.2 µs, p95 41332.4 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 279.9,
            "unit": "µs",
            "extra": "min 270.1 µs, p95 301.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 408.2,
            "unit": "µs",
            "extra": "min 356 µs, p95 440.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 403.8,
            "unit": "µs",
            "extra": "min 392.3 µs, p95 427 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.18,
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
            "value": 0.179,
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
            "value": 6.63,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.98,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 24.55,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.29,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "d206f4557fe71d1eaa3005d3c0b2a60e7a2f30c0",
          "message": "fix: S1.3 the broker client retries a 429 only when the broker says when\n\nPactFlow answers 429 when a publication would create an integration past the plan's limit, and\nno retry gets past that. The client now retries a 429 only when it carries Retry-After, after that\ndelay; a 5xx keeps the client's own backoff unless the broker asks for a delay of its own; and a\nRetry-After longer than 10 seconds is reported at once, with the delay, rather than waited for.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T14:42:11+11:00",
          "tree_id": "3cc7f258c0fd9a2fa803dfd33485166be3ee95e6",
          "url": "https://github.com/rholshausen/pact-janus/commit/d206f4557fe71d1eaa3005d3c0b2a60e7a2f30c0"
        },
        "date": 1792382200805,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.8,
            "unit": "µs",
            "extra": "min 8.6 µs, p95 11.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2016.8,
            "unit": "µs",
            "extra": "min 1965.4 µs, p95 2095.2 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 260.9,
            "unit": "µs",
            "extra": "min 206 µs, p95 953.6 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2288,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7107621,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33863,
            "unit": "µs",
            "extra": "min 33294.1 µs, p95 34996.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35592,
            "unit": "µs",
            "extra": "min 34199.6 µs, p95 37340.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40659.5,
            "unit": "µs",
            "extra": "min 40290.5 µs, p95 41282.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 433,
            "unit": "µs",
            "extra": "min 415.9 µs, p95 452.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 545,
            "unit": "µs",
            "extra": "min 509 µs, p95 568.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 489,
            "unit": "µs",
            "extra": "min 462.4 µs, p95 510 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16571,
            "unit": "µs",
            "extra": "min 15909.2 µs, p95 18455.9 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18766.3,
            "unit": "µs",
            "extra": "min 18484.1 µs, p95 19098.9 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.5,
            "unit": "µs",
            "extra": "min 18.9 µs, p95 20.1 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26.2,
            "unit": "µs",
            "extra": "min 25.9 µs, p95 26.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 23408.3,
            "unit": "µs",
            "extra": "min 22008.1 µs, p95 25735.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 23413.6,
            "unit": "µs",
            "extra": "min 21304.7 µs, p95 26923.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12794.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12741.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 135.3,
            "unit": "µs",
            "extra": "min 116.4 µs, p95 140.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 175.5,
            "unit": "µs",
            "extra": "min 153.8 µs, p95 192.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 22.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 56,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 137.1,
            "unit": "µs",
            "extra": "min 117.7 µs, p95 161.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 176.8,
            "unit": "µs",
            "extra": "min 136.1 µs, p95 189.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 67.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 365.3,
            "unit": "µs",
            "extra": "min 293.9 µs, p95 562.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 481.1,
            "unit": "µs",
            "extra": "min 366.2 µs, p95 650.9 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33204.6,
            "unit": "µs",
            "extra": "min 32398.7 µs, p95 33804.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34421.6,
            "unit": "µs",
            "extra": "min 33379.8 µs, p95 35911.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40873.1,
            "unit": "µs",
            "extra": "min 40512.8 µs, p95 41669.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 283.2,
            "unit": "µs",
            "extra": "min 273.9 µs, p95 309.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 415.1,
            "unit": "µs",
            "extra": "min 359.2 µs, p95 438.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 403.7,
            "unit": "µs",
            "extra": "min 391.9 µs, p95 426.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.24,
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
            "value": 0.35,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.186,
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
            "value": 6.44,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.908,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.78,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 26.13,
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
            "value": 0.53,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "2afc74f288b61ccda9d1ad2d9d7a91067b6a79b6",
          "message": "test: S1.3 the broker client replays a scenario recorded from the pinned broker\n\nThe second test layer: one scenario (a v3 pact and a Janus contract published, fetched with\npending and WIP, results published, fetched again, a changed republish refused) recorded from\nPact Broker 2.121.2 into engine/broker-client/tests/recorded/, and replayed offline on every\nsystem: each request must be the recorded one, and each step's conclusion the recorded one. The\nbroker's URL, timestamps and database ids are normalised, so a recording is the same file however\noften it is made; CI's verifier-diff job records it again and fails on any difference.\n\nThe hand-written cases from pact_verifier's own list: a body that is not JSON, a broker too old\nfor pacts-for-verification, a selection that is not HAL or selects nothing, selectors refused with\na 400, and a broker at a context path.\n\nThey found a difference, fixed here: today's verifier fetches every link from the broker it was\ngiven, keeping the link's path and query and adding a missing context path, so a broker behind a\nproxy that writes an internal host still works. The client now does the same, with pact_verifier's\nown cases (normalize_path_inner, resolve_path_test).\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T15:26:49+11:00",
          "tree_id": "f2e2ee5f9b0d7926c29e603c69a17b25406d7f0d",
          "url": "https://github.com/rholshausen/pact-janus/commit/2afc74f288b61ccda9d1ad2d9d7a91067b6a79b6"
        },
        "date": 1792384246016,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 7.2,
            "unit": "µs",
            "extra": "min 6.8 µs, p95 9.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1794.3,
            "unit": "µs",
            "extra": "min 1739.3 µs, p95 1877.3 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 141.3,
            "unit": "µs",
            "extra": "min 106.3 µs, p95 371 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1720,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7107621,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 29532.5,
            "unit": "µs",
            "extra": "min 28961.2 µs, p95 31472.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 30991.7,
            "unit": "µs",
            "extra": "min 29707.3 µs, p95 32781.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 34192.8,
            "unit": "µs",
            "extra": "min 33852.3 µs, p95 34602.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 339.4,
            "unit": "µs",
            "extra": "min 326.7 µs, p95 353.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 390.4,
            "unit": "µs",
            "extra": "min 354.4 µs, p95 422.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 354.3,
            "unit": "µs",
            "extra": "min 337.1 µs, p95 377.9 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13923.3,
            "unit": "µs",
            "extra": "min 13265.3 µs, p95 15226 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 14195.9,
            "unit": "µs",
            "extra": "min 13942 µs, p95 14530.6 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 15.5,
            "unit": "µs",
            "extra": "min 15 µs, p95 15.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.7,
            "unit": "µs",
            "extra": "min 20.1 µs, p95 21.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 19881.6,
            "unit": "µs",
            "extra": "min 18662.7 µs, p95 22343.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 21144.3,
            "unit": "µs",
            "extra": "min 18709.3 µs, p95 25413.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 11202.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12150.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 89.5,
            "unit": "µs",
            "extra": "min 82.9 µs, p95 103.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 105.9,
            "unit": "µs",
            "extra": "min 96 µs, p95 123.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 18.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 40.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 75.5,
            "unit": "µs",
            "extra": "min 69.3 µs, p95 91.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 101.7,
            "unit": "µs",
            "extra": "min 87.3 µs, p95 115.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 13.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 36.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 272.6,
            "unit": "µs",
            "extra": "min 238.3 µs, p95 389.2 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 375.8,
            "unit": "µs",
            "extra": "min 312.3 µs, p95 454.2 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 28941,
            "unit": "µs",
            "extra": "min 28286.4 µs, p95 29847.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 29363.4,
            "unit": "µs",
            "extra": "min 28614.4 µs, p95 30754.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 34795.4,
            "unit": "µs",
            "extra": "min 34355.7 µs, p95 35126.8 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 210.8,
            "unit": "µs",
            "extra": "min 203.2 µs, p95 228.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 295.6,
            "unit": "µs",
            "extra": "min 285.2 µs, p95 314.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 306.8,
            "unit": "µs",
            "extra": "min 296.6 µs, p95 333.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.6,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.8,
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
            "value": 0.129,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.138,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 4.97,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.686,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 21.05,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 21.5,
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
          "id": "2e219d8173b7410b070925f94f74d56ff93e4c0c",
          "message": "test: S1.3 the broker client's own pact with the Pact Broker, verified by the broker\n\nThe third test layer: engine/broker-client/tests/pact.rs writes the client's consumer pact with\nthe provider \"Pact Broker\" (pact_consumer) and checks the client against it on every system; in\nCI's verifier-diff job today's verifier verifies it against the pinned broker, each provider state\nset up through the broker's API by an endpoint of the test's own. Eleven interactions: the index;\npacts for verification with a pending pact and with a work-in-progress one, and fetching each; the\nprovider, its branch and tag; a verification result; publishing contracts, and a changed republish\nrefused. The parties and state names are the Ruby broker's own (Condor, the Pricing Service), so the\npact can be proposed upstream as it stands. Requests are exact; responses match by type, on only\nwhat the client reads.\n\npact_consumer is pinned to 1.4.10, the release that reads the pact_models the kernel uses, with no\ndefault features (no plugin driver). pact_models moves from 1.3.14 to 1.3.15, whose one change is\nthe fix for interactions duplicated on merge (pact-reference #550), which the pact's scenarios hit;\nthe kernel reads pacts and never merges them, and the corpora and the differential harness are\nunchanged. .cargo/config.toml sets PACT_DO_NOT_TRACK, so no test run sends pact_consumer's usage\nmetrics.\n\nNo-Changeset: test-only, and the one shipped dependency that moves changes only pact merging, which\nthe engine never does.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T15:54:05+11:00",
          "tree_id": "d40971a3d72ecdfa9b2ad6937ee2efbd4ad5b94e",
          "url": "https://github.com/rholshausen/pact-janus/commit/2e219d8173b7410b070925f94f74d56ff93e4c0c"
        },
        "date": 1792385924022,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.1,
            "unit": "µs",
            "extra": "min 6.7 µs, p95 10 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1563.4,
            "unit": "µs",
            "extra": "min 1452.6 µs, p95 1709 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 104.9,
            "unit": "µs",
            "extra": "min 91.3 µs, p95 139 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2055.3,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7195817,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 29873.7,
            "unit": "µs",
            "extra": "min 28305.6 µs, p95 32355.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 32910.5,
            "unit": "µs",
            "extra": "min 30341.9 µs, p95 35973.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 34185,
            "unit": "µs",
            "extra": "min 31816.8 µs, p95 36410.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 393.1,
            "unit": "µs",
            "extra": "min 366.7 µs, p95 443.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 468.5,
            "unit": "µs",
            "extra": "min 396.3 µs, p95 547.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 426.6,
            "unit": "µs",
            "extra": "min 392.6 µs, p95 478.3 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 15871.4,
            "unit": "µs",
            "extra": "min 15248.8 µs, p95 17260.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 14370.8,
            "unit": "µs",
            "extra": "min 13830.8 µs, p95 15957.9 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 16.6,
            "unit": "µs",
            "extra": "min 15.8 µs, p95 18.5 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.1,
            "unit": "µs",
            "extra": "min 18.8 µs, p95 23.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 24419.6,
            "unit": "µs",
            "extra": "min 21197.1 µs, p95 26286.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24551.9,
            "unit": "µs",
            "extra": "min 21562.3 µs, p95 26670.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 14618,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 14391.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 98.7,
            "unit": "µs",
            "extra": "min 85.4 µs, p95 111 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 118.4,
            "unit": "µs",
            "extra": "min 100.7 µs, p95 141.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 29.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 44.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 90.9,
            "unit": "µs",
            "extra": "min 75.9 µs, p95 107.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 121.6,
            "unit": "µs",
            "extra": "min 93.6 µs, p95 149 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 19,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 49.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 333,
            "unit": "µs",
            "extra": "min 268.4 µs, p95 519.8 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 448.6,
            "unit": "µs",
            "extra": "min 284.4 µs, p95 522 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 29476.1,
            "unit": "µs",
            "extra": "min 27642 µs, p95 32445.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 32764.9,
            "unit": "µs",
            "extra": "min 29912.1 µs, p95 35790.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 33250.5,
            "unit": "µs",
            "extra": "min 31533.8 µs, p95 35223.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 271.9,
            "unit": "µs",
            "extra": "min 240.8 µs, p95 297.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 374.7,
            "unit": "µs",
            "extra": "min 304.1 µs, p95 563.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 340.2,
            "unit": "µs",
            "extra": "min 321.2 µs, p95 376.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.83,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.34,
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
            "value": 0.33,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.146,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.197,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 5.34,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.73,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.1,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 26.19,
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
            "value": 0.3,
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
            "value": 0.45,
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
          "id": "060c3c4f132d653c7d7f1ecf9b285b17fe18ad21",
          "message": "docs: S1.3 drops the schema lint against PactFlow's OpenAPI document\n\nLayers 1 to 3 test against the real OSS broker: every request the client sends is compared with\ntoday's verifier's or sent exactly to the pinned broker, which validates it, and selectors are the\nuser's, passed through as written. The lint's own gaps (PactFlow's document omits fetching a pact\nand publishing results, and could only be fetched, not committed) made it the weakest layer. What\nis given up is any test of PactFlow's own behaviour; PactFlow runs the same broker beneath.\n\nThe exit no longer names the lint, and the text says what is still to build from layer 1: the run\nbehind basic auth and a bearer token, and the weekly run against the latest image.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T15:57:42+11:00",
          "tree_id": "4f60fdc29a4477775cc9a436862a090e502ef7c8",
          "url": "https://github.com/rholshausen/pact-janus/commit/060c3c4f132d653c7d7f1ecf9b285b17fe18ad21"
        },
        "date": 1792386203023,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 9,
            "unit": "µs",
            "extra": "min 8.6 µs, p95 11.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2049.3,
            "unit": "µs",
            "extra": "min 1993.5 µs, p95 2070 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 197.4,
            "unit": "µs",
            "extra": "min 173 µs, p95 259 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2325.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7195817,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33594.9,
            "unit": "µs",
            "extra": "min 33171.2 µs, p95 34189.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 34932.8,
            "unit": "µs",
            "extra": "min 34364.1 µs, p95 35656.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40415.6,
            "unit": "µs",
            "extra": "min 40154.3 µs, p95 40860.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 432.1,
            "unit": "µs",
            "extra": "min 411.9 µs, p95 459.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 514.5,
            "unit": "µs",
            "extra": "min 465 µs, p95 549.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 491,
            "unit": "µs",
            "extra": "min 464.4 µs, p95 514.7 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16081.7,
            "unit": "µs",
            "extra": "min 15741.2 µs, p95 16726.2 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19196.8,
            "unit": "µs",
            "extra": "min 18762 µs, p95 19547.8 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.4,
            "unit": "µs",
            "extra": "min 18.9 µs, p95 20 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.6,
            "unit": "µs",
            "extra": "min 25.2 µs, p95 26.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22635.7,
            "unit": "µs",
            "extra": "min 21964.9 µs, p95 23722.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 23030.4,
            "unit": "µs",
            "extra": "min 22071.1 µs, p95 25552.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12176.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12534.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 149.2,
            "unit": "µs",
            "extra": "min 141.5 µs, p95 158.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 184.3,
            "unit": "µs",
            "extra": "min 160.3 µs, p95 197.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 27.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 71.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 135.6,
            "unit": "µs",
            "extra": "min 116.9 µs, p95 146.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 172.9,
            "unit": "µs",
            "extra": "min 142.2 µs, p95 187.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 62.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 403.7,
            "unit": "µs",
            "extra": "min 247.7 µs, p95 538.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 474.6,
            "unit": "µs",
            "extra": "min 394.3 µs, p95 700.3 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 32954.4,
            "unit": "µs",
            "extra": "min 32660 µs, p95 33383.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34157.6,
            "unit": "µs",
            "extra": "min 33540.4 µs, p95 34736.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40416.4,
            "unit": "µs",
            "extra": "min 40053.2 µs, p95 40840 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 280.1,
            "unit": "µs",
            "extra": "min 272.4 µs, p95 305.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 417.1,
            "unit": "µs",
            "extra": "min 407.1 µs, p95 440.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 408.1,
            "unit": "µs",
            "extra": "min 395 µs, p95 431.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.18,
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
            "value": 0.33,
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
            "value": 0.208,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.58,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.951,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 24.7,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.87,
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
            "value": 0.5,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "a534cd50ddce11104fb6f8dd37a3816ec6468fa4",
          "message": "test: S1.3 a broker credential reaches the broker and appears nowhere else\n\nADR 0032 decision 5, end to end: cli/tests/cli.rs puts the pinned broker behind a door that\ndemands basic credentials or a bearer token (the OSS broker has no bearer tokens, so the door is a\nproxy for both), runs janus publish and janus verify --publish through it at trace, in text and\nJSON, and with a wrong credential, and fails if the secret or the Authorization header it became\nappears in anything printed. CI's broker step runs it.\n\nIt found two leaks, neither released:\n\n- the engine's trace log of each frame repeated the broker descriptor's auth; a logged frame now\n  keeps only an auth member's kind, and a url loses its user information;\n- the HTTP library's trace output dumps raw request bytes, Authorization included; janus and\n  janus-engine hold ureq_proto at debug, where headers are redacted, whatever is asked for.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T16:09:14+11:00",
          "tree_id": "18a513f78373ed5d21e87ddea39eac0cdea41293",
          "url": "https://github.com/rholshausen/pact-janus/commit/a534cd50ddce11104fb6f8dd37a3816ec6468fa4"
        },
        "date": 1792386816976,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.9,
            "unit": "µs",
            "extra": "min 8.5 µs, p95 18.2 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2016.9,
            "unit": "µs",
            "extra": "min 1950.4 µs, p95 2093.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 178.9,
            "unit": "µs",
            "extra": "min 143.3 µs, p95 256.8 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2297.7,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7089638,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34231.6,
            "unit": "µs",
            "extra": "min 33627.2 µs, p95 34990.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35291.5,
            "unit": "µs",
            "extra": "min 34524.5 µs, p95 36306.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41584.7,
            "unit": "µs",
            "extra": "min 41025.6 µs, p95 42048.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 426.2,
            "unit": "µs",
            "extra": "min 411.3 µs, p95 446.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 501.2,
            "unit": "µs",
            "extra": "min 463.5 µs, p95 530.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 486.1,
            "unit": "µs",
            "extra": "min 462.2 µs, p95 512.1 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16571.5,
            "unit": "µs",
            "extra": "min 15857.9 µs, p95 18556.3 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18897.1,
            "unit": "µs",
            "extra": "min 18379.2 µs, p95 19731.5 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.1,
            "unit": "µs",
            "extra": "min 18.7 µs, p95 19.6 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.8,
            "unit": "µs",
            "extra": "min 25.5 µs, p95 26.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 24615.7,
            "unit": "µs",
            "extra": "min 22202.8 µs, p95 25795 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24095.7,
            "unit": "µs",
            "extra": "min 22593.1 µs, p95 31400.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13639.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13319.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 143.3,
            "unit": "µs",
            "extra": "min 131.9 µs, p95 151.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 171.6,
            "unit": "µs",
            "extra": "min 137.3 µs, p95 185.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 25.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 54.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 140.9,
            "unit": "µs",
            "extra": "min 124.1 µs, p95 164 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 169.9,
            "unit": "µs",
            "extra": "min 143.5 µs, p95 181.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 27.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 62.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 422.3,
            "unit": "µs",
            "extra": "min 331.2 µs, p95 469.9 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 489.5,
            "unit": "µs",
            "extra": "min 400.7 µs, p95 672.2 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33644,
            "unit": "µs",
            "extra": "min 33118.1 µs, p95 34802.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34376.1,
            "unit": "µs",
            "extra": "min 33718.3 µs, p95 35207.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41193.3,
            "unit": "µs",
            "extra": "min 40739 µs, p95 41766.8 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 279.6,
            "unit": "µs",
            "extra": "min 269.1 µs, p95 301.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 415.5,
            "unit": "µs",
            "extra": "min 344.8 µs, p95 445.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 404.6,
            "unit": "µs",
            "extra": "min 391.7 µs, p95 426.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.19,
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
            "value": 0.22,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.56,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.925,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.35,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 27.37,
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
            "value": 0.42,
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
          "id": "9317b79cb3da763b95b8af99de00d9fc0d6d9173",
          "message": "feat: S1.3 is done: the broker checks run weekly against the latest Pact Broker\n\nThe Latest Pact Broker workflow runs every check CI runs against the pinned broker against\npactfoundation/pact-broker:latest, weekly and on demand: the differential harness (--any-version),\nthe live publish, verify and credentials tests, the client's pact verified by the broker, and the\nrecording made again and compared except for its version line. Every check runs even when one\nbefore it failed, and the run's summary names the broker's version and digest beside the pin.\nNothing it does changes the repository: moving the pin stays a deliberate change.\n\npact-broker.sh takes PACT_BROKER_IMAGE and PACT_BROKER_CONTAINER, so another image can run beside\nthe pinned one, and the recording test takes JANUS_PACT_BROKER_ANY_VERSION=1. Rehearsed locally\nagainst a second broker; today the latest image is the pinned one.\n\nS1.3's Done line records what its exit did not foresee and what was left out.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T16:23:03+11:00",
          "tree_id": "0c1ff57df891723e91f2aad6b4b2c2d87279c92b",
          "url": "https://github.com/rholshausen/pact-janus/commit/9317b79cb3da763b95b8af99de00d9fc0d6d9173"
        },
        "date": 1792387628193,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 7.3,
            "unit": "µs",
            "extra": "min 6.8 µs, p95 9.4 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1780.5,
            "unit": "µs",
            "extra": "min 1732.2 µs, p95 1872.5 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 180.3,
            "unit": "µs",
            "extra": "min 140.2 µs, p95 204.9 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1733,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7089638,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 31241.8,
            "unit": "µs",
            "extra": "min 29547 µs, p95 32654.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 29942.7,
            "unit": "µs",
            "extra": "min 29288.5 µs, p95 31115.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 34359.5,
            "unit": "µs",
            "extra": "min 33563.4 µs, p95 35220.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 340.9,
            "unit": "µs",
            "extra": "min 328.3 µs, p95 354.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 374.4,
            "unit": "µs",
            "extra": "min 350 µs, p95 399.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 354.1,
            "unit": "µs",
            "extra": "min 335.4 µs, p95 374.6 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 14639.5,
            "unit": "µs",
            "extra": "min 13339.7 µs, p95 16891.5 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 14362.5,
            "unit": "µs",
            "extra": "min 14044.8 µs, p95 14900.8 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 15.6,
            "unit": "µs",
            "extra": "min 15.1 µs, p95 15.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.2,
            "unit": "µs",
            "extra": "min 19.7 µs, p95 20.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 20974.5,
            "unit": "µs",
            "extra": "min 18629.9 µs, p95 23056.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 22116.6,
            "unit": "µs",
            "extra": "min 19592.2 µs, p95 25907.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12179.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 12847,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 80.7,
            "unit": "µs",
            "extra": "min 71.8 µs, p95 94.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 109.2,
            "unit": "µs",
            "extra": "min 93.3 µs, p95 122.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 14.9,
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
            "value": 79,
            "unit": "µs",
            "extra": "min 70.4 µs, p95 92.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 111.9,
            "unit": "µs",
            "extra": "min 94.3 µs, p95 127.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 14.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 40.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 281,
            "unit": "µs",
            "extra": "min 238.5 µs, p95 344.7 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 417.6,
            "unit": "µs",
            "extra": "min 344.3 µs, p95 613.3 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 30204.1,
            "unit": "µs",
            "extra": "min 29101.6 µs, p95 32138 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 29460.4,
            "unit": "µs",
            "extra": "min 28837.2 µs, p95 33087.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 34593.4,
            "unit": "µs",
            "extra": "min 34125.6 µs, p95 35013 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 212.4,
            "unit": "µs",
            "extra": "min 204.5 µs, p95 234.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 261.7,
            "unit": "µs",
            "extra": "min 249.9 µs, p95 315.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 301.5,
            "unit": "µs",
            "extra": "min 293.5 µs, p95 315.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.65,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.81,
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
            "value": 0.134,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.14,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 5.23,
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
            "value": 21.21,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 23.67,
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
            "value": 0.33,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.44,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.45,
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
          "id": "3302d687e7dd764544b8ecb4c9439cc391ac87cb",
          "message": "fix: the rename rehearsal rewraps the CLI's help and renames recorded contracts\n\nThe weekly rehearsal failed on two tests after S1.3.\n\nThe CLI's help is wrapped to a width, so a name of another length breaks its\nlines at other words and the trycmd snapshots no longer match. The rehearsal\nnow rewrites them from the renamed CLI and fails unless they read word for word\nas the renamed snapshots did; Documentation/codename.md adds the same step to\nrenaming for real.\n\nThe broker client's recording held each published contract as the base64 the\nbroker's API takes, so the rename could not reach the codename in its\n$format. The recording now holds the contract decoded, re-recorded against the\npinned broker.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T09:26:56+11:00",
          "tree_id": "9694828db237c187314688ce8a046001e8e69605",
          "url": "https://github.com/rholshausen/pact-janus/commit/3302d687e7dd764544b8ecb4c9439cc391ac87cb"
        },
        "date": 1792449638395,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 14,
            "unit": "µs",
            "extra": "min 8.6 µs, p95 20.6 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2106,
            "unit": "µs",
            "extra": "min 2018.1 µs, p95 2204.8 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 261.2,
            "unit": "µs",
            "extra": "min 178.8 µs, p95 378.3 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2312.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7089638,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 35021,
            "unit": "µs",
            "extra": "min 34001.4 µs, p95 36247.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 37569.5,
            "unit": "µs",
            "extra": "min 35322.5 µs, p95 38898.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41067.7,
            "unit": "µs",
            "extra": "min 40662.4 µs, p95 41675.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 431.2,
            "unit": "µs",
            "extra": "min 414.6 µs, p95 453.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 509.7,
            "unit": "µs",
            "extra": "min 469.8 µs, p95 564 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 481.1,
            "unit": "µs",
            "extra": "min 452.9 µs, p95 506.7 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18671.5,
            "unit": "µs",
            "extra": "min 16573.8 µs, p95 20027.4 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18649.3,
            "unit": "µs",
            "extra": "min 18107.1 µs, p95 19279.6 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.8,
            "unit": "µs",
            "extra": "min 19.3 µs, p95 20.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.4,
            "unit": "µs",
            "extra": "min 25.1 µs, p95 26.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 25214.9,
            "unit": "µs",
            "extra": "min 23354.1 µs, p95 26949.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24723.7,
            "unit": "µs",
            "extra": "min 22070.9 µs, p95 27707 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13961.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13792.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 145.9,
            "unit": "µs",
            "extra": "min 130 µs, p95 157 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 173.1,
            "unit": "µs",
            "extra": "min 160.8 µs, p95 185.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 27.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 55.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 140.4,
            "unit": "µs",
            "extra": "min 124.6 µs, p95 147.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 176.4,
            "unit": "µs",
            "extra": "min 150.5 µs, p95 187.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 26.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 68.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 392.2,
            "unit": "µs",
            "extra": "min 278.3 µs, p95 549.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 525.3,
            "unit": "µs",
            "extra": "min 451.2 µs, p95 631.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34662.6,
            "unit": "µs",
            "extra": "min 33599.6 µs, p95 35712.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35467.2,
            "unit": "µs",
            "extra": "min 34022.8 µs, p95 37625.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41228,
            "unit": "µs",
            "extra": "min 40642.5 µs, p95 41691.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 284.1,
            "unit": "µs",
            "extra": "min 273.2 µs, p95 309.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 362.8,
            "unit": "µs",
            "extra": "min 352.8 µs, p95 381.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 399.4,
            "unit": "µs",
            "extra": "min 383.7 µs, p95 426.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.17,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.57,
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
            "value": 0.18,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.209,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.24,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.891,
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
            "value": 29.04,
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
          "id": "9a95033c168eddc13908d1c03920c73e345cf93b",
          "message": "feat: S1.3 verify's broker credentials also answer to --broker-*, and verify has short flags\n\n--user, --password and --token keep today's verifier spelling (S1.13), with\n--broker-user, --broker-password and --broker-token as visible aliases, so the help\nsays they are the broker's credentials and not the provider's. --log-level gets -l\n(today's verifier's short flag), verify's --provider-url -u and --output -o, and a\nbare `janus verify` prints its help, exit 2.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T10:32:33+11:00",
          "tree_id": "2af4533108b9b42353930c9d06a3f3910da97b72",
          "url": "https://github.com/rholshausen/pact-janus/commit/9a95033c168eddc13908d1c03920c73e345cf93b"
        },
        "date": 1792453019442,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.1,
            "unit": "µs",
            "extra": "min 7.9 µs, p95 10.4 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1575.4,
            "unit": "µs",
            "extra": "min 1479.5 µs, p95 1841.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 139.8,
            "unit": "µs",
            "extra": "min 97.8 µs, p95 785.4 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2280.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7089638,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34999.4,
            "unit": "µs",
            "extra": "min 33707.6 µs, p95 36056.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 37309.5,
            "unit": "µs",
            "extra": "min 35778.9 µs, p95 38228.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 42579.5,
            "unit": "µs",
            "extra": "min 40855 µs, p95 43408.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 445.8,
            "unit": "µs",
            "extra": "min 427.9 µs, p95 461 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 513.5,
            "unit": "µs",
            "extra": "min 452 µs, p95 550.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 510,
            "unit": "µs",
            "extra": "min 484.8 µs, p95 525.6 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17532.8,
            "unit": "µs",
            "extra": "min 16702.9 µs, p95 18968.3 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 17950.6,
            "unit": "µs",
            "extra": "min 17740.8 µs, p95 18453.9 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19,
            "unit": "µs",
            "extra": "min 18.6 µs, p95 19.4 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.7,
            "unit": "µs",
            "extra": "min 25 µs, p95 26.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 28694.2,
            "unit": "µs",
            "extra": "min 26655.4 µs, p95 30386.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 27812.2,
            "unit": "µs",
            "extra": "min 25331.4 µs, p95 36783.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 16999.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 16190.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 111.9,
            "unit": "µs",
            "extra": "min 100.2 µs, p95 122.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 126.2,
            "unit": "µs",
            "extra": "min 110.6 µs, p95 138.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 26,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 47.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 97.3,
            "unit": "µs",
            "extra": "min 86 µs, p95 112.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 122.3,
            "unit": "µs",
            "extra": "min 104 µs, p95 139 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 18.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 51.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 283,
            "unit": "µs",
            "extra": "min 242.1 µs, p95 396.5 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 416.6,
            "unit": "µs",
            "extra": "min 368.2 µs, p95 531.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33695.1,
            "unit": "µs",
            "extra": "min 32071.7 µs, p95 34781.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35198.3,
            "unit": "µs",
            "extra": "min 33920.7 µs, p95 36470.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 42988.5,
            "unit": "µs",
            "extra": "min 41747.6 µs, p95 43701.9 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 289.9,
            "unit": "µs",
            "extra": "min 277 µs, p95 303.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 333.6,
            "unit": "µs",
            "extra": "min 315 µs, p95 355.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 401.9,
            "unit": "µs",
            "extra": "min 386.9 µs, p95 431.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.98,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.18,
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
            "value": 0.32,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.165,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.178,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 5.53,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.755,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 28.44,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 27.78,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.32,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.33,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.49,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "0d958a95297e857f9bd7b92e1e9c75b7172b6e96",
          "message": "feat: every janus command that takes arguments prints its help when given none\n\nclap does not propagate arg_required_else_help from the top-level command, so\ncheck, explain, upgrade, publish, component push, component pull and completions\neach set it, as verify does. The exit code stays 2. usage.md pins each bare\ncommand's first line; help.md already pins the text whole.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T10:36:07+11:00",
          "tree_id": "8585e801b33b92d7674b41aea07d2f05db703ba7",
          "url": "https://github.com/rholshausen/pact-janus/commit/0d958a95297e857f9bd7b92e1e9c75b7172b6e96"
        },
        "date": 1792453254055,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 6.9,
            "unit": "µs",
            "extra": "min 6.6 µs, p95 9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1736.7,
            "unit": "µs",
            "extra": "min 1696.1 µs, p95 1802.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 179.6,
            "unit": "µs",
            "extra": "min 130.4 µs, p95 200.9 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1726.7,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7089638,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 29229.6,
            "unit": "µs",
            "extra": "min 28303.3 µs, p95 30081.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 30985.7,
            "unit": "µs",
            "extra": "min 29684.4 µs, p95 32184.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 33686.6,
            "unit": "µs",
            "extra": "min 33429.9 µs, p95 34073.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 340.5,
            "unit": "µs",
            "extra": "min 327.5 µs, p95 355 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 370.4,
            "unit": "µs",
            "extra": "min 343.7 µs, p95 402 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 356.9,
            "unit": "µs",
            "extra": "min 339.2 µs, p95 381.4 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13605,
            "unit": "µs",
            "extra": "min 13142.1 µs, p95 15588.1 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 14236.4,
            "unit": "µs",
            "extra": "min 14049.7 µs, p95 14428.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 15.3,
            "unit": "µs",
            "extra": "min 14.9 µs, p95 15.7 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.2,
            "unit": "µs",
            "extra": "min 19.6 µs, p95 20.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 18572.7,
            "unit": "µs",
            "extra": "min 17932.8 µs, p95 21439.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 20125.3,
            "unit": "µs",
            "extra": "min 19018.7 µs, p95 22429.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 10423.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 11290.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 77.6,
            "unit": "µs",
            "extra": "min 73.3 µs, p95 93.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 107.1,
            "unit": "µs",
            "extra": "min 90.3 µs, p95 121.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 13.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 38.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 76.5,
            "unit": "µs",
            "extra": "min 70.9 µs, p95 90.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 100.4,
            "unit": "µs",
            "extra": "min 88.4 µs, p95 113.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 14.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 37.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 268,
            "unit": "µs",
            "extra": "min 235.2 µs, p95 371.6 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 361.1,
            "unit": "µs",
            "extra": "min 309.8 µs, p95 423.9 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 29063.6,
            "unit": "µs",
            "extra": "min 28076 µs, p95 29851.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 29456,
            "unit": "µs",
            "extra": "min 28616.3 µs, p95 30770.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 34220.8,
            "unit": "µs",
            "extra": "min 33690.6 µs, p95 34835.8 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 207,
            "unit": "µs",
            "extra": "min 199.4 µs, p95 220.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 291.1,
            "unit": "µs",
            "extra": "min 280.8 µs, p95 306.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 297.5,
            "unit": "µs",
            "extra": "min 289.6 µs, p95 313.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.59,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.77,
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
            "value": 0.128,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.136,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 4.99,
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
            "value": 20.57,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 21.56,
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
            "value": 0.32,
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
            "value": 0.45,
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
          "id": "80c4c55a76eab95cc5065f5c5dd18f8327da6b01",
          "message": "fix: S1.3 the HTTP library's logging is capped at debug, not raised to it\n\nwithout_credentials added `ureq_proto=debug` to the EnvFilter, and a directive\nsets a target's level, so at --log-level warn every request logged its debug\nlines. The cap is now a second filter an event must also pass: the HTTP library\nlogs at the level asked for, and never at trace, where it dumps request bytes.\nBoth binaries install the one subscriber in cli/src/logging.rs, and its test\npins the levels.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T10:42:59+11:00",
          "tree_id": "8b1595c86e71ebda1a7717f7c7b646f29fcda553",
          "url": "https://github.com/rholshausen/pact-janus/commit/80c4c55a76eab95cc5065f5c5dd18f8327da6b01"
        },
        "date": 1792453606639,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 7.5,
            "unit": "µs",
            "extra": "min 6.7 µs, p95 12.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1822.1,
            "unit": "µs",
            "extra": "min 1771.9 µs, p95 1916.3 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 172.6,
            "unit": "µs",
            "extra": "min 149.6 µs, p95 208.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1736.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7089638,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 28930.7,
            "unit": "µs",
            "extra": "min 28080.4 µs, p95 30466.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 31422.9,
            "unit": "µs",
            "extra": "min 29905.5 µs, p95 32467.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 34628.6,
            "unit": "µs",
            "extra": "min 34107.7 µs, p95 36157.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 339.7,
            "unit": "µs",
            "extra": "min 325.7 µs, p95 354.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 403.8,
            "unit": "µs",
            "extra": "min 350.3 µs, p95 467.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 358.9,
            "unit": "µs",
            "extra": "min 338.2 µs, p95 383 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13462.9,
            "unit": "µs",
            "extra": "min 12920.3 µs, p95 14588.6 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 13994.6,
            "unit": "µs",
            "extra": "min 13737.8 µs, p95 14382 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 15.3,
            "unit": "µs",
            "extra": "min 14.8 µs, p95 15.7 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.2,
            "unit": "µs",
            "extra": "min 19.7 µs, p95 21 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 19427.7,
            "unit": "µs",
            "extra": "min 18338.2 µs, p95 20674.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 19693.2,
            "unit": "µs",
            "extra": "min 18250.3 µs, p95 22769.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 11088.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 11080.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 91,
            "unit": "µs",
            "extra": "min 87.6 µs, p95 175.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 109.2,
            "unit": "µs",
            "extra": "min 100.1 µs, p95 237.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 19,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 39.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 78.2,
            "unit": "µs",
            "extra": "min 69.6 µs, p95 185.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 102.3,
            "unit": "µs",
            "extra": "min 79 µs, p95 233 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 14.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 38.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 286.1,
            "unit": "µs",
            "extra": "min 242.8 µs, p95 481.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 438.6,
            "unit": "µs",
            "extra": "min 349.5 µs, p95 682.5 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 28125.2,
            "unit": "µs",
            "extra": "min 27535.5 µs, p95 29414.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 30123.3,
            "unit": "µs",
            "extra": "min 28879.6 µs, p95 31995.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 34828.4,
            "unit": "µs",
            "extra": "min 33957.1 µs, p95 35586.2 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 207.9,
            "unit": "µs",
            "extra": "min 200.6 µs, p95 223.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 256.7,
            "unit": "µs",
            "extra": "min 248.8 µs, p95 340.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 330.5,
            "unit": "µs",
            "extra": "min 290.2 µs, p95 553.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.72,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.12,
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
            "value": 0.28,
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
            "value": 0.174,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 5.53,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.778,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 21.81,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 22.42,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.32,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.33,
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
            "value": 0.46,
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
          "id": "7135a2c76676e54b7566fd4c21347b58c610c627",
          "message": "feat: explain and verify --explain-failures colour the plan on a terminal\n\nPlan-grammar spec §3.3 (new): a host may style either text form on a terminal,\nso long as removing the styling leaves the form exactly; nothing recorded or read\nby a program is styled, and the engine never styles.\n\nThe kernel reads a rendered plan back as tokens (plan::tokens), beside the\nrenderer; a test reads every corpus snapshot with it, nothing lost and nothing\nunrecognised. The CLI styles the tokens (src/highlight.rs) and writes them\nthrough anstream, already in the tree behind clap: colour on a terminal unless\nNO_COLOR or TERM=dumb, anywhere with CLICOLOR_FORCE. The CLI's tests now run\nwithout colour whatever CLICOLOR_FORCE says, which also fixes help.md's\nsnapshots when it is set.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T11:07:58+11:00",
          "tree_id": "7e4a4ff4ca37d36d7bcfd70713dd0f2679d0c748",
          "url": "https://github.com/rholshausen/pact-janus/commit/7135a2c76676e54b7566fd4c21347b58c610c627"
        },
        "date": 1792455151343,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 17.2,
            "unit": "µs",
            "extra": "min 8.8 µs, p95 27.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2026.7,
            "unit": "µs",
            "extra": "min 1981.6 µs, p95 2127.4 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 262.9,
            "unit": "µs",
            "extra": "min 189.5 µs, p95 310.7 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2391.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7087491,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33905.1,
            "unit": "µs",
            "extra": "min 33178.1 µs, p95 35102.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35695.1,
            "unit": "µs",
            "extra": "min 34884.2 µs, p95 37181.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40718.9,
            "unit": "µs",
            "extra": "min 40182.6 µs, p95 41352.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 431.5,
            "unit": "µs",
            "extra": "min 413.6 µs, p95 455.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 519.3,
            "unit": "µs",
            "extra": "min 469.7 µs, p95 555.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 478.9,
            "unit": "µs",
            "extra": "min 457.1 µs, p95 497.1 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16768.6,
            "unit": "µs",
            "extra": "min 16012.8 µs, p95 18421.1 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19010.7,
            "unit": "µs",
            "extra": "min 18408.7 µs, p95 19753.2 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.8,
            "unit": "µs",
            "extra": "min 19.2 µs, p95 20.2 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.7,
            "unit": "µs",
            "extra": "min 25.4 µs, p95 26.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 25355.2,
            "unit": "µs",
            "extra": "min 23230 µs, p95 27014.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 25911.4,
            "unit": "µs",
            "extra": "min 22522.7 µs, p95 31491.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 14091.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 14197.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 148.4,
            "unit": "µs",
            "extra": "min 123.2 µs, p95 157 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 185.7,
            "unit": "µs",
            "extra": "min 154.6 µs, p95 201.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 27.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 68.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 138.6,
            "unit": "µs",
            "extra": "min 120.1 µs, p95 150 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 171.7,
            "unit": "µs",
            "extra": "min 148.9 µs, p95 184.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 23.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 63.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 377.5,
            "unit": "µs",
            "extra": "min 273.5 µs, p95 638.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 552.1,
            "unit": "µs",
            "extra": "min 421.7 µs, p95 756.3 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 32637.5,
            "unit": "µs",
            "extra": "min 32125.1 µs, p95 33817.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34839,
            "unit": "µs",
            "extra": "min 33819.1 µs, p95 35975 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40927,
            "unit": "µs",
            "extra": "min 40316.4 µs, p95 41442.4 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 286.8,
            "unit": "µs",
            "extra": "min 276.6 µs, p95 312.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 418.4,
            "unit": "µs",
            "extra": "min 357.9 µs, p95 442.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 402.6,
            "unit": "µs",
            "extra": "min 389.7 µs, p95 422.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.22,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.5,
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
            "value": 0.186,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.205,
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
            "value": 0.884,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 26.3,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 27.18,
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
          "id": "0f33682e29bee578028e7d6385b50767054ce136",
          "message": "fix: the broker recording normalises the engine version in the broker's echo of a result\n\nThe recording wrote the engine's version as {engine-version} in the\nverification result the client publishes, but not in the broker's 201\nanswer, which echoes verifiedBy back. A release moving the version\ntherefore changed the recording, and CI's re-record failed on the\nrelease PR. Response bodies now get the same normalisation.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T11:50:53+11:00",
          "tree_id": "de3eabcb4f4f53a850182539c82b2ee342414dc8",
          "url": "https://github.com/rholshausen/pact-janus/commit/0f33682e29bee578028e7d6385b50767054ce136"
        },
        "date": 1792458341645,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 9.4,
            "unit": "µs",
            "extra": "min 9.1 µs, p95 11.6 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2114.9,
            "unit": "µs",
            "extra": "min 2026.7 µs, p95 2207.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 246.8,
            "unit": "µs",
            "extra": "min 184.5 µs, p95 313.9 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2287.5,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7087491,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34391.4,
            "unit": "µs",
            "extra": "min 33041.9 µs, p95 35803 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35727.2,
            "unit": "µs",
            "extra": "min 34595.6 µs, p95 37764.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40702.5,
            "unit": "µs",
            "extra": "min 40193.6 µs, p95 41384.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 426.9,
            "unit": "µs",
            "extra": "min 410.6 µs, p95 447.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 506.2,
            "unit": "µs",
            "extra": "min 466.1 µs, p95 543.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 477.3,
            "unit": "µs",
            "extra": "min 453.6 µs, p95 499.8 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16570.7,
            "unit": "µs",
            "extra": "min 15917.1 µs, p95 19398.8 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18970.6,
            "unit": "µs",
            "extra": "min 18372.8 µs, p95 19826 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 18.9,
            "unit": "µs",
            "extra": "min 18.4 µs, p95 19.4 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.8,
            "unit": "µs",
            "extra": "min 25.4 µs, p95 26.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22984.4,
            "unit": "µs",
            "extra": "min 21649.5 µs, p95 25563.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24267.2,
            "unit": "µs",
            "extra": "min 22551.6 µs, p95 27479.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12494.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13455.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 140.5,
            "unit": "µs",
            "extra": "min 127.7 µs, p95 149.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 179.1,
            "unit": "µs",
            "extra": "min 158.4 µs, p95 189.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 23.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 64.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 134.8,
            "unit": "µs",
            "extra": "min 122 µs, p95 143.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 176,
            "unit": "µs",
            "extra": "min 148.7 µs, p95 189.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.3,
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
            "value": 385.9,
            "unit": "µs",
            "extra": "min 276.1 µs, p95 475.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 510.1,
            "unit": "µs",
            "extra": "min 434.5 µs, p95 633.3 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33306.3,
            "unit": "µs",
            "extra": "min 32660.5 µs, p95 34520.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34613.9,
            "unit": "µs",
            "extra": "min 33852.3 µs, p95 36989.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41020.2,
            "unit": "µs",
            "extra": "min 40341.2 µs, p95 41776.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 280.3,
            "unit": "µs",
            "extra": "min 271.1 µs, p95 307 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 371.7,
            "unit": "µs",
            "extra": "min 352.8 µs, p95 432.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 406.9,
            "unit": "µs",
            "extra": "min 391.3 µs, p95 443.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.17,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.5,
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
            "value": 0.178,
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
            "value": 6.67,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.953,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 24.72,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.91,
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
            "value": 0.54,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "79f09d6af69b2a1f0044e77474fc5f38490aa6ed",
          "message": "chore: release 0.2.0",
          "timestamp": "2026-10-20T12:21:26+11:00",
          "tree_id": "362620dfa584c8ad505abb2dbf71c4f5ef01d478",
          "url": "https://github.com/rholshausen/pact-janus/commit/79f09d6af69b2a1f0044e77474fc5f38490aa6ed"
        },
        "date": 1792459556757,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 9.5,
            "unit": "µs",
            "extra": "min 8.6 µs, p95 16.5 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2280.8,
            "unit": "µs",
            "extra": "min 2207.4 µs, p95 2337.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 179.5,
            "unit": "µs",
            "extra": "min 138.4 µs, p95 548.3 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2221.2,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7087387,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 37688.1,
            "unit": "µs",
            "extra": "min 36205.5 µs, p95 39063.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 38990.2,
            "unit": "µs",
            "extra": "min 37998 µs, p95 39937.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 43056,
            "unit": "µs",
            "extra": "min 42764.1 µs, p95 43398.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 442.2,
            "unit": "µs",
            "extra": "min 424 µs, p95 457.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 524.9,
            "unit": "µs",
            "extra": "min 456.8 µs, p95 565.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 456.6,
            "unit": "µs",
            "extra": "min 431.4 µs, p95 481.6 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 19331.3,
            "unit": "µs",
            "extra": "min 17269.6 µs, p95 20927.6 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 17979.3,
            "unit": "µs",
            "extra": "min 17652.7 µs, p95 18525.2 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.5,
            "unit": "µs",
            "extra": "min 18.9 µs, p95 19.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26.4,
            "unit": "µs",
            "extra": "min 25.7 µs, p95 27.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 24270,
            "unit": "µs",
            "extra": "min 23269.7 µs, p95 26795.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24408.7,
            "unit": "µs",
            "extra": "min 22971.6 µs, p95 26378.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13598.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13613.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 102.3,
            "unit": "µs",
            "extra": "min 95.1 µs, p95 124.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 137.6,
            "unit": "µs",
            "extra": "min 121.5 µs, p95 158.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 16.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 49.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 105.4,
            "unit": "µs",
            "extra": "min 89 µs, p95 124.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 132.2,
            "unit": "µs",
            "extra": "min 118.7 µs, p95 150.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 18.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 51.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 351.9,
            "unit": "µs",
            "extra": "min 280.6 µs, p95 727.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 523.9,
            "unit": "µs",
            "extra": "min 408.7 µs, p95 2022.5 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 37009.2,
            "unit": "µs",
            "extra": "min 35501.1 µs, p95 38300.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 39477.1,
            "unit": "µs",
            "extra": "min 37343.7 µs, p95 40983 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 43560.8,
            "unit": "µs",
            "extra": "min 43250.6 µs, p95 44105.1 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 275.1,
            "unit": "µs",
            "extra": "min 266.4 µs, p95 291.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 339.5,
            "unit": "µs",
            "extra": "min 319.9 µs, p95 409.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 388.8,
            "unit": "µs",
            "extra": "min 377 µs, p95 405 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.09,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.34,
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
            "value": 0.167,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.184,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.57,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.915,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 39.06,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.13,
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
          "id": "ced015bd006e654d93bc09dfa45907cc7fdedf57",
          "message": "fix: the Janus benchmarks' lockfile records component-http's tracing dependency\n\nbenchmarks/janus is its own workspace with its own Cargo.lock, which bb162a0\ndid not update, so the Benchmarks workflow's --locked build failed.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T13:41:18+11:00",
          "tree_id": "c2375c84ad4b885f732b108997426d2878b59300",
          "url": "https://github.com/rholshausen/pact-janus/commit/ced015bd006e654d93bc09dfa45907cc7fdedf57"
        },
        "date": 1792464978588,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 15,
            "unit": "µs",
            "extra": "min 9.1 µs, p95 24.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2163.6,
            "unit": "µs",
            "extra": "min 2115.2 µs, p95 2251.1 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 182.1,
            "unit": "µs",
            "extra": "min 160.1 µs, p95 1256.8 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2305.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7087387,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34986.2,
            "unit": "µs",
            "extra": "min 33461.4 µs, p95 36318.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36152.7,
            "unit": "µs",
            "extra": "min 35303.5 µs, p95 37348 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 42007.6,
            "unit": "µs",
            "extra": "min 41615.1 µs, p95 42478.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 430.8,
            "unit": "µs",
            "extra": "min 414.5 µs, p95 458 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 497.2,
            "unit": "µs",
            "extra": "min 464.8 µs, p95 538.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 483.1,
            "unit": "µs",
            "extra": "min 461.1 µs, p95 509.6 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17792,
            "unit": "µs",
            "extra": "min 16417.9 µs, p95 19722.6 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18743.6,
            "unit": "µs",
            "extra": "min 18410.9 µs, p95 19679.5 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.8,
            "unit": "µs",
            "extra": "min 19.3 µs, p95 20.4 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.7,
            "unit": "µs",
            "extra": "min 25.4 µs, p95 26.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 25058.6,
            "unit": "µs",
            "extra": "min 23056.4 µs, p95 29373.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24453,
            "unit": "µs",
            "extra": "min 22581.4 µs, p95 28039.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13955.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13694.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 122,
            "unit": "µs",
            "extra": "min 108.7 µs, p95 152.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 146.3,
            "unit": "µs",
            "extra": "min 140.6 µs, p95 184.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 21.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 53.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 112,
            "unit": "µs",
            "extra": "min 104.7 µs, p95 148.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 139.4,
            "unit": "µs",
            "extra": "min 131.2 µs, p95 177.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 21.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 49.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 406.7,
            "unit": "µs",
            "extra": "min 319.1 µs, p95 526 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 552.3,
            "unit": "µs",
            "extra": "min 476.8 µs, p95 739.8 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34245.5,
            "unit": "µs",
            "extra": "min 33038.7 µs, p95 35642.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35252.4,
            "unit": "µs",
            "extra": "min 34380.2 µs, p95 36665.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 42324,
            "unit": "µs",
            "extra": "min 41718.1 µs, p95 42984.9 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 284.7,
            "unit": "µs",
            "extra": "min 276 µs, p95 311.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 384.5,
            "unit": "µs",
            "extra": "min 365.4 µs, p95 417.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 405.9,
            "unit": "µs",
            "extra": "min 391.7 µs, p95 426.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.21,
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
            "value": 0.35,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.18,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.213,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.8,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 1.02,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 28.71,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.02,
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
            "value": 0.59,
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
          "id": "6ca0f39929209673c5ce93f226b454caea18eb74",
          "message": "fix: a contract's executed plan shows the response it was scored on\n\nA shape-compiled plan carries both parts, but verification resolves it against\nthe provider's reply alone, so the request half of the executed plan resolved\nevery path to absence and printed as failing. Only the response is scored, so\nthe verification/executed-plan event's text is now that subtree: the same as a\nv1–v4 pact's, and the plan that decided the verdict.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T15:21:40+11:00",
          "tree_id": "ec7bead8a2e9776ea8d82a4dd606f77d48b62264",
          "url": "https://github.com/rholshausen/pact-janus/commit/6ca0f39929209673c5ce93f226b454caea18eb74"
        },
        "date": 1792470363252,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.8,
            "unit": "µs",
            "extra": "min 8.6 µs, p95 12.4 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2148.9,
            "unit": "µs",
            "extra": "min 2080 µs, p95 2231.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 178.3,
            "unit": "µs",
            "extra": "min 162.4 µs, p95 216.5 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2332.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7087394,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33965.5,
            "unit": "µs",
            "extra": "min 33475.8 µs, p95 34542.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35773.4,
            "unit": "µs",
            "extra": "min 35064.5 µs, p95 36507.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41278.1,
            "unit": "µs",
            "extra": "min 40641.5 µs, p95 42252.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 430.1,
            "unit": "µs",
            "extra": "min 415 µs, p95 450.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 505.2,
            "unit": "µs",
            "extra": "min 458.4 µs, p95 600.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 486,
            "unit": "µs",
            "extra": "min 463.3 µs, p95 507.2 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16387.3,
            "unit": "µs",
            "extra": "min 16010.1 µs, p95 17484.2 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18337.8,
            "unit": "µs",
            "extra": "min 18131 µs, p95 18883.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.6,
            "unit": "µs",
            "extra": "min 19.1 µs, p95 20.1 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.9,
            "unit": "µs",
            "extra": "min 25.6 µs, p95 26.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 23417.2,
            "unit": "µs",
            "extra": "min 22123.7 µs, p95 25254.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24457.3,
            "unit": "µs",
            "extra": "min 21949.1 µs, p95 27030 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12858.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13680.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 121,
            "unit": "µs",
            "extra": "min 109.2 µs, p95 152.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 144.6,
            "unit": "µs",
            "extra": "min 138 µs, p95 182.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 22,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 51.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 111.1,
            "unit": "µs",
            "extra": "min 105.5 µs, p95 145.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 139.5,
            "unit": "µs",
            "extra": "min 131.2 µs, p95 175.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 21.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 51.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 404.3,
            "unit": "µs",
            "extra": "min 299.6 µs, p95 584.6 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 505.8,
            "unit": "µs",
            "extra": "min 458.9 µs, p95 739.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33280.3,
            "unit": "µs",
            "extra": "min 32795 µs, p95 34114.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35208.6,
            "unit": "µs",
            "extra": "min 34439.4 µs, p95 36409.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41240.9,
            "unit": "µs",
            "extra": "min 40537.2 µs, p95 41897 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 282.4,
            "unit": "µs",
            "extra": "min 273.4 µs, p95 307.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 368.3,
            "unit": "µs",
            "extra": "min 326.4 µs, p95 420.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 406.1,
            "unit": "µs",
            "extra": "min 393.7 µs, p95 427.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.48,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.56,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.33,
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
            "value": 0.194,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.208,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.27,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.871,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 27.26,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 28.21,
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
            "value": 0.41,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.58,
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
          "id": "5db014a1a66d0ab9a7be1220bf817209f4b04e0c",
          "message": "feat: S1.4 a v1–v4 pact's request generators are applied in verification\n\nADR 0034. The v1–v4 generator vocabulary is the generator component `pact`,\nbuilt into the kernel behind generator/generate and producing values with\npact_models, as today's verifier does. Verifying a pact generates the request's\npath, headers, query and JSON body after state setup and before before-request;\nresponse generators are not applied, as today.\n\nA state-setup hook result carries `values`, and the pact-state-change format\nreads a JSON object reply as them, so a state endpoint that returns {\"id\": 1234}\nfeeds ${id} unchanged. A generator that produces nothing keeps the recorded\nvalue, as today, and the run emits a generator-failed verification/warning.\n\nS1.1's corpus gains four generators/ fixtures (pact-reference's issue #441 case,\na state-change reply into a path and header, every value generator from the\ncompatibility suite, and an expression with nothing to read); the harness's\nroutes gain `patterns` and its state callback a `values` answer. All agree with\ntoday's verifier; janus from before this disagreed on all but the last.\n\nSpecs: lifecycle-hooks §3.2, §4.4, §8.4, §10.2; component-interfaces §7.3 and\n§11.1 (unknown-generator, generate-failed); the verification/warning event,\nemitted since the prototype, now in the protocol's event table and schema, with\nthe bindings regenerated. Generators in contracts are the new item S1.19.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T16:33:02+11:00",
          "tree_id": "f9cb2b823af26259e7c5031b2521e8165178036a",
          "url": "https://github.com/rholshausen/pact-janus/commit/5db014a1a66d0ab9a7be1220bf817209f4b04e0c"
        },
        "date": 1792475185656,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 9.4,
            "unit": "µs",
            "extra": "min 9.2 µs, p95 11.6 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2050.7,
            "unit": "µs",
            "extra": "min 2016.3 µs, p95 2167.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 162.9,
            "unit": "µs",
            "extra": "min 137.1 µs, p95 259.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2462.3,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7662366,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 33153.5,
            "unit": "µs",
            "extra": "min 32586.3 µs, p95 34317.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 34884.4,
            "unit": "µs",
            "extra": "min 34101.1 µs, p95 35666.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 40498.6,
            "unit": "µs",
            "extra": "min 40221.3 µs, p95 40978 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 430,
            "unit": "µs",
            "extra": "min 410.7 µs, p95 448.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 542.1,
            "unit": "µs",
            "extra": "min 470.3 µs, p95 572 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 483.1,
            "unit": "µs",
            "extra": "min 458.2 µs, p95 506.8 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 15972.8,
            "unit": "µs",
            "extra": "min 15555.5 µs, p95 16674.9 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18331.9,
            "unit": "µs",
            "extra": "min 18129.9 µs, p95 18505.5 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.2,
            "unit": "µs",
            "extra": "min 18.7 µs, p95 19.7 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.8,
            "unit": "µs",
            "extra": "min 25.6 µs, p95 26.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22049.9,
            "unit": "µs",
            "extra": "min 21417.7 µs, p95 23465.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 22248.2,
            "unit": "µs",
            "extra": "min 21457 µs, p95 24993.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 11746.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 11959,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 145.6,
            "unit": "µs",
            "extra": "min 130.6 µs, p95 153 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 176.8,
            "unit": "µs",
            "extra": "min 151.5 µs, p95 182.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 27.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 58.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 134.3,
            "unit": "µs",
            "extra": "min 115.2 µs, p95 145.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 170.2,
            "unit": "µs",
            "extra": "min 138.1 µs, p95 184.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 22.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 61.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 378.6,
            "unit": "µs",
            "extra": "min 298.1 µs, p95 563.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 465.6,
            "unit": "µs",
            "extra": "min 393.6 µs, p95 641.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 32481.6,
            "unit": "µs",
            "extra": "min 32110.4 µs, p95 33549.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34260,
            "unit": "µs",
            "extra": "min 33456.7 µs, p95 35169.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40320.4,
            "unit": "µs",
            "extra": "min 39967.8 µs, p95 40852.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 277,
            "unit": "µs",
            "extra": "min 268.5 µs, p95 300.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 414.6,
            "unit": "µs",
            "extra": "min 399.5 µs, p95 439.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 401.8,
            "unit": "µs",
            "extra": "min 388.4 µs, p95 423.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.17,
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
            "value": 0.195,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 6.3,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.888,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 25.03,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.69,
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
            "value": 0.41,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "ca5162bc2013ce0e535fcab4ee8116079b4a82e2",
          "message": "feat: S1.5 providers and brokers are reached through one async HTTP client\n\nADR 0035. engine/http-client is the one client the HTTP transport's drive role\nand the broker client send through: reqwest on rustls with ring and the platform\nverifier, today's verifier's stack, already in the tree through pact_models. It\nruns on a current-thread runtime with names resolved inside the request, so it\nstarts no thread; a Linux test counts the process's threads around a request.\n\nOne document configures it, the HTTP client options (component-interfaces spec\n§5.6, http-transport.schema.json): TLS roots, a client certificate and an\ninsecure mode; proxies (the environment's by default, one named, or none);\nrequest and connect timeouts; HTTP/1.1, HTTP/2 or negotiated; retries. It is the\ndrive options' `client` and the broker descriptor's. The drive options add\n`base-url`, whose path is the base path, and `headers`, which replace the\ninteraction's own of the same name in any case. Certificates and keys are PEM\ntext; the frame trace log hides a key and every provider header value.\n\njanus verify takes today's --base-path, --header, --request-timeout and\n--disable-ssl-verification, with their environment variables, and adds\n--ca-cert, --client-cert/--client-key, --proxy, --no-proxy, --connect-timeout\nand --http-version. Documentation/verifier-migration.md is S1.13's table, with\nevery transport option of both of today's verifiers filled in.\n\nBreaking: verification's sends carry no timeout-ms, so a provider request is\nbounded by the transport's request timeout, 5 s by default as today, where the\nkernel's was 30 s. A variant whose request could not be sent now carries the\nprotocol's component-failed error, with a message saying why.\n\nThe exit's verification is in cli/tests/cli.rs: a TLS provider demanding a\nclient certificate, through a proxy, at trace, printing neither the key nor the\nheader's token. tools/test-servers holds the authority, TLS provider and proxy\nthe tests share. S1.1's corpus gains six connection/ fixtures; five agree with\ntoday's verifier, and connection/header-case is a deliberate difference in the\nledger. The whole corpus changed no verdict.\n\nFound: the prototype's client asked providers for gzip and trusted its own\nbundled roots, both now as today's verifier. The hooks' trace-log redaction gap\nand the http hook's client are added to S1.9.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T10:56:06+11:00",
          "tree_id": "3535aae04eccead1522f2807fa684102e705db69",
          "url": "https://github.com/rholshausen/pact-janus/commit/ca5162bc2013ce0e535fcab4ee8116079b4a82e2"
        },
        "date": 1792540996100,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 6.8,
            "unit": "µs",
            "extra": "min 6.6 µs, p95 9.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1919.2,
            "unit": "µs",
            "extra": "min 1859.3 µs, p95 2029.4 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 182.8,
            "unit": "µs",
            "extra": "min 136.9 µs, p95 203.1 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1902.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7660485,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 29548.5,
            "unit": "µs",
            "extra": "min 28644.1 µs, p95 31588.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 31315.3,
            "unit": "µs",
            "extra": "min 30025.8 µs, p95 34245 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 34369.3,
            "unit": "µs",
            "extra": "min 34117.8 µs, p95 34927.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 337.7,
            "unit": "µs",
            "extra": "min 325.2 µs, p95 351.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 400.4,
            "unit": "µs",
            "extra": "min 351.1 µs, p95 446.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 368.6,
            "unit": "µs",
            "extra": "min 346.7 µs, p95 395.3 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13602.3,
            "unit": "µs",
            "extra": "min 13144.7 µs, p95 15332.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 14293.5,
            "unit": "µs",
            "extra": "min 14081.7 µs, p95 14498 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 15.4,
            "unit": "µs",
            "extra": "min 15 µs, p95 15.8 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.8,
            "unit": "µs",
            "extra": "min 20.3 µs, p95 21.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 19739.9,
            "unit": "µs",
            "extra": "min 17989.4 µs, p95 22243.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 19457.5,
            "unit": "µs",
            "extra": "min 17958.6 µs, p95 21480.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 11167.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 11132.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 70.9,
            "unit": "µs",
            "extra": "min 68.1 µs, p95 87.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 97.6,
            "unit": "µs",
            "extra": "min 88.7 µs, p95 130.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 13.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 35.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 68.1,
            "unit": "µs",
            "extra": "min 64.7 µs, p95 92.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 90.7,
            "unit": "µs",
            "extra": "min 85.2 µs, p95 108.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 13.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 34.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 275.5,
            "unit": "µs",
            "extra": "min 239.3 µs, p95 409.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 352.8,
            "unit": "µs",
            "extra": "min 320.8 µs, p95 404.9 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 29246.2,
            "unit": "µs",
            "extra": "min 28035.6 µs, p95 30639.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 31158.7,
            "unit": "µs",
            "extra": "min 29301.8 µs, p95 33124.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 34971.6,
            "unit": "µs",
            "extra": "min 34636.7 µs, p95 35497.8 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 211.2,
            "unit": "µs",
            "extra": "min 202.8 µs, p95 226.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 251.7,
            "unit": "µs",
            "extra": "min 242.9 µs, p95 267.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 303.9,
            "unit": "µs",
            "extra": "min 295.1 µs, p95 318.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.64,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.97,
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
            "value": 0.158,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 9.79,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.667,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 27.51,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 24.77,
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
            "value": 0.28,
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
            "value": 0.4,
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
          "id": "8f2789c0c390af20d98972eb841056127336dde9",
          "message": "docs: split S1.6 into a body kind per item, S1.6 and S1.20 to S1.24\n\nS1.6 was every body kind at once (L). It is now plain-text and binary bodies\n(S), the two that need nothing new in the document model. Member order and the\nlegacy body compiler's retyping, which form and XML both need, become S1.20,\nafter S1.11. Form-urlencoded and XML follow as S1.21 and S1.22, and multipart,\nwhich compares each part through the others' components, comes last as S1.23.\nEach closes its own S1.1 ledger entries, so each can be marked done on its own.\n\nMeasured first: of the 158 skipped pact-specification cases, 156 are XML, and\ntwo carry a janus:skip blaming unanchored regexes, which S1.18 anchored; both\npass without it, and S1.6 removes the markers. So the \"no janus:skip left\" exit\nmoves to S1.22. The four matching rules the legacy compiler refuses (Values,\nEachKey, EachValue, ArrayContains) were in S1.6's findings row though no case\ncovers them; they are S1.24.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T11:05:31+11:00",
          "tree_id": "cb9c99ff5af450259a97322c11fe5b00ef19b83d",
          "url": "https://github.com/rholshausen/pact-janus/commit/8f2789c0c390af20d98972eb841056127336dde9"
        },
        "date": 1792541434194,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 9.3,
            "unit": "µs",
            "extra": "min 9 µs, p95 43 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2319.7,
            "unit": "µs",
            "extra": "min 2259.5 µs, p95 2414.2 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 271.2,
            "unit": "µs",
            "extra": "min 222.8 µs, p95 295.7 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2592.5,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7660485,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34880.1,
            "unit": "µs",
            "extra": "min 33969.4 µs, p95 36212.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36452.6,
            "unit": "µs",
            "extra": "min 35471.1 µs, p95 39319.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41169.8,
            "unit": "µs",
            "extra": "min 40736.3 µs, p95 41541.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 436.2,
            "unit": "µs",
            "extra": "min 416.2 µs, p95 458.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 532,
            "unit": "µs",
            "extra": "min 459.7 µs, p95 571.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 496.2,
            "unit": "µs",
            "extra": "min 467.3 µs, p95 535.5 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17881.1,
            "unit": "µs",
            "extra": "min 16290 µs, p95 19902.9 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18723.3,
            "unit": "µs",
            "extra": "min 18362.9 µs, p95 19538.7 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.7,
            "unit": "µs",
            "extra": "min 19.3 µs, p95 20.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26.4,
            "unit": "µs",
            "extra": "min 26 µs, p95 27.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 25193.2,
            "unit": "µs",
            "extra": "min 22816.8 µs, p95 27458 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 25910.8,
            "unit": "µs",
            "extra": "min 24018.5 µs, p95 32885.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 14039.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 14427.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 125.2,
            "unit": "µs",
            "extra": "min 120.7 µs, p95 160.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 146.8,
            "unit": "µs",
            "extra": "min 139.2 µs, p95 190.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 26.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 53,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 111.1,
            "unit": "µs",
            "extra": "min 105.5 µs, p95 146.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 144.1,
            "unit": "µs",
            "extra": "min 132.8 µs, p95 187.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 21.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 53,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 411.5,
            "unit": "µs",
            "extra": "min 320.6 µs, p95 522.9 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 526.7,
            "unit": "µs",
            "extra": "min 448.2 µs, p95 590 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34570.6,
            "unit": "µs",
            "extra": "min 33140.6 µs, p95 35506.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35211,
            "unit": "µs",
            "extra": "min 34392.4 µs, p95 36409 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41646.9,
            "unit": "µs",
            "extra": "min 40931.3 µs, p95 42125.7 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 283.4,
            "unit": "µs",
            "extra": "min 275.4 µs, p95 307.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 345.4,
            "unit": "µs",
            "extra": "min 329.7 µs, p95 379.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 412,
            "unit": "µs",
            "extra": "min 396.2 µs, p95 432.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.21,
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
            "value": 0.204,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 10.81,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.765,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 29.32,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 30.93,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.35,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.38,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.48,
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
          "id": "28e86c853aa12361c481a1da856cc62409dafadd",
          "message": "fix: the SDK bindings carry S1.5's schema descriptions\n\nS1.5 reworded the broker descriptor's `client` description and described a\ntransport binding's `options`, pointing both at the HTTP client options. The\ngenerated TypeScript and JVM bindings carry descriptions as doc comments, and\nwere not regenerated, so CI's \"Bindings match their schemas\" step failed.\nRegenerated with `cargo run -p pact_janus_bindings -- generate`; doc comments\nonly.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T11:24:12+11:00",
          "tree_id": "372773487eefbbd740a43dbb933fe6d25d8a58c8",
          "url": "https://github.com/rholshausen/pact-janus/commit/28e86c853aa12361c481a1da856cc62409dafadd"
        },
        "date": 1792543140988,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10,
            "unit": "µs",
            "extra": "min 9.6 µs, p95 12.5 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2331.2,
            "unit": "µs",
            "extra": "min 2259.9 µs, p95 2466.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 188.8,
            "unit": "µs",
            "extra": "min 169.5 µs, p95 235.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2520.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7660485,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 35549.2,
            "unit": "µs",
            "extra": "min 34087.6 µs, p95 36872.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 38389.2,
            "unit": "µs",
            "extra": "min 37011.2 µs, p95 39363.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41702.3,
            "unit": "µs",
            "extra": "min 41122.1 µs, p95 42453.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 434.1,
            "unit": "µs",
            "extra": "min 414 µs, p95 464.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 499,
            "unit": "µs",
            "extra": "min 459 µs, p95 573.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 493,
            "unit": "µs",
            "extra": "min 466.5 µs, p95 517.9 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 22029,
            "unit": "µs",
            "extra": "min 17972.5 µs, p95 23244.4 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19283.6,
            "unit": "µs",
            "extra": "min 18448.3 µs, p95 20400.8 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.8,
            "unit": "µs",
            "extra": "min 19.3 µs, p95 20.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.8,
            "unit": "µs",
            "extra": "min 25.5 µs, p95 26.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 26771.7,
            "unit": "µs",
            "extra": "min 24467.6 µs, p95 29314.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 26935.4,
            "unit": "µs",
            "extra": "min 24812.4 µs, p95 29898.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 15132,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 15239.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 121.9,
            "unit": "µs",
            "extra": "min 111.7 µs, p95 151.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 145,
            "unit": "µs",
            "extra": "min 137.9 µs, p95 185.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 24.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 52.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 119.3,
            "unit": "µs",
            "extra": "min 105.7 µs, p95 154.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 140.2,
            "unit": "µs",
            "extra": "min 129.5 µs, p95 178.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 25.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 47.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 408.1,
            "unit": "µs",
            "extra": "min 334.3 µs, p95 567.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 469.8,
            "unit": "µs",
            "extra": "min 415.8 µs, p95 761.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34987.6,
            "unit": "µs",
            "extra": "min 33676.2 µs, p95 36391.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 36640.2,
            "unit": "µs",
            "extra": "min 35216.1 µs, p95 38139 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 42138.1,
            "unit": "µs",
            "extra": "min 41470.6 µs, p95 43092 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 286.9,
            "unit": "µs",
            "extra": "min 275.3 µs, p95 322.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 390.7,
            "unit": "µs",
            "extra": "min 371.7 µs, p95 424.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 405.6,
            "unit": "µs",
            "extra": "min 391 µs, p95 435.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.22,
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
            "value": 0.35,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.184,
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
            "value": 11.1,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.826,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 29.62,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 31.94,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.36,
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
            "value": 0.48,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "3412e5c41fd6ad192af6a7b784f11bba769d3aeb",
          "message": "fix: a request the provider could not answer is reported once, on one line\n\njanus verify printed a variant's component-failed error with its details as a\nJSON line beneath it, and the engine logged the same failure again at warn, in\nRust's debug format. The line now ends with the component's own code in\nbrackets, which is all the JSON added; --output json still carries the whole\nerror. The engine logs a failed variant, a transport failure or mismatches, at\ndebug, since the run's result event reports it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T11:40:33+11:00",
          "tree_id": "0b9538c84e3b3a88efabffefe1133de1f624d152",
          "url": "https://github.com/rholshausen/pact-janus/commit/3412e5c41fd6ad192af6a7b784f11bba769d3aeb"
        },
        "date": 1792543464930,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 7.1,
            "unit": "µs",
            "extra": "min 6.7 µs, p95 21.1 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1812.9,
            "unit": "µs",
            "extra": "min 1735 µs, p95 1914.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 161.4,
            "unit": "µs",
            "extra": "min 117.9 µs, p95 280 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1943.2,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7660485,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 28481,
            "unit": "µs",
            "extra": "min 28020.6 µs, p95 29025.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 31063.7,
            "unit": "µs",
            "extra": "min 30278.6 µs, p95 32285.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 34526.7,
            "unit": "µs",
            "extra": "min 33690.9 µs, p95 35161.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 335,
            "unit": "µs",
            "extra": "min 323.6 µs, p95 347.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 380.9,
            "unit": "µs",
            "extra": "min 354.2 µs, p95 421.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 359,
            "unit": "µs",
            "extra": "min 337 µs, p95 384.2 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13472.7,
            "unit": "µs",
            "extra": "min 12891.5 µs, p95 14337 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 14312.4,
            "unit": "µs",
            "extra": "min 13910.5 µs, p95 15180.6 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 15.1,
            "unit": "µs",
            "extra": "min 14.7 µs, p95 16.4 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 20.3,
            "unit": "µs",
            "extra": "min 19.7 µs, p95 20.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 18273.6,
            "unit": "µs",
            "extra": "min 17724.9 µs, p95 19019.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 20533.2,
            "unit": "µs",
            "extra": "min 18206.4 µs, p95 23161.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 10054.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 11588.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 79.2,
            "unit": "µs",
            "extra": "min 72.4 µs, p95 92.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 105.4,
            "unit": "µs",
            "extra": "min 94.9 µs, p95 120 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 13,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 37.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 76.6,
            "unit": "µs",
            "extra": "min 69.8 µs, p95 88.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 101.8,
            "unit": "µs",
            "extra": "min 86.8 µs, p95 116.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 14.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 37.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 269.8,
            "unit": "µs",
            "extra": "min 220.1 µs, p95 368.7 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 382.8,
            "unit": "µs",
            "extra": "min 337 µs, p95 490.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 27976.4,
            "unit": "µs",
            "extra": "min 27612.7 µs, p95 28644 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 29241.1,
            "unit": "µs",
            "extra": "min 28410.8 µs, p95 30145 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 34996.8,
            "unit": "µs",
            "extra": "min 34125 µs, p95 36308.5 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 209.5,
            "unit": "µs",
            "extra": "min 199.3 µs, p95 222.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 254.3,
            "unit": "µs",
            "extra": "min 243 µs, p95 273.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 303.2,
            "unit": "µs",
            "extra": "min 291 µs, p95 320.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.6,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.81,
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
            "value": 0.26,
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
            "value": 0.139,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 8.83,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.588,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 22.77,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 22.56,
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
            "value": 0.29,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.36,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.39,
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
          "id": "379f7ff5398a4899c9464109b13829ed677ccd36",
          "message": "refactor: S1.11 the v1–v4 compatibility layer is the kernel's legacy module\n\nKernel-boundary review findings 3 and 6: nothing in the module tree marked the\nv1–v4 pact reader and matching-rule compiler as the permanent HTTP- and\nJSON-shaped exception to \"the kernel knows nothing about HTTP or JSON\".\n\nengine/kernel/src/legacy/ is that boundary, and its mod.rs says so: pact (was\nlegacy_pact.rs), compile (was plan/legacy.rs) and upgrade (was upgrade.rs,\nwhich needs the same knowledge). A module, not a crate: the compiler builds the\nkernel's plan nodes and verification calls the compiler, so each would need the\nother.\n\nengine/kernel/tests/boundary.rs holds the rest of the kernel to it, in cargo\ntest on every system: outside legacy/, any pact_models use fails unless its\nlist allows it, and it allows two, neither HTTP nor a pact document (the\nbuilt-in `pact` generator component's vocabulary, and the datetime operator's\nformat check). S1.4 had put a pact_models request in the verification path for\nits generators; it is now an opaque legacy::pact::RequestTemplate.\n\nAGENTS.md's architecture rule names the exception. No behaviour changes.\n\nNo-Changeset: a move inside the kernel; nothing an engine pin sees changes.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T12:05:35+11:00",
          "tree_id": "c66c1525b5116af4af30f88c5757ce6403cd3578",
          "url": "https://github.com/rholshausen/pact-janus/commit/379f7ff5398a4899c9464109b13829ed677ccd36"
        },
        "date": 1792545009235,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.9,
            "unit": "µs",
            "extra": "min 8.4 µs, p95 11.6 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2237.6,
            "unit": "µs",
            "extra": "min 2153.1 µs, p95 2318.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 231.5,
            "unit": "µs",
            "extra": "min 159.6 µs, p95 276.5 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2558.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7663326,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34392.7,
            "unit": "µs",
            "extra": "min 33555.8 µs, p95 35043.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35627.8,
            "unit": "µs",
            "extra": "min 34790.1 µs, p95 36616 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41584.8,
            "unit": "µs",
            "extra": "min 41292.8 µs, p95 41973.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 431.1,
            "unit": "µs",
            "extra": "min 415 µs, p95 452.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 542.4,
            "unit": "µs",
            "extra": "min 499.2 µs, p95 580.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 493.9,
            "unit": "µs",
            "extra": "min 468.2 µs, p95 523.8 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17782.6,
            "unit": "µs",
            "extra": "min 16499.3 µs, p95 18729.4 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18995.2,
            "unit": "µs",
            "extra": "min 18359.4 µs, p95 19556.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.4,
            "unit": "µs",
            "extra": "min 18.8 µs, p95 19.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26,
            "unit": "µs",
            "extra": "min 25.6 µs, p95 26.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 24524.9,
            "unit": "µs",
            "extra": "min 22218.3 µs, p95 26697.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24758.4,
            "unit": "µs",
            "extra": "min 23313.1 µs, p95 27097.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13739.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13754.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 123.5,
            "unit": "µs",
            "extra": "min 110.8 µs, p95 158.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 144.7,
            "unit": "µs",
            "extra": "min 138.2 µs, p95 181.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 27,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 52.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 119.8,
            "unit": "µs",
            "extra": "min 106.3 µs, p95 152.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 140.6,
            "unit": "µs",
            "extra": "min 125.8 µs, p95 176.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 26.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 52,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 380.3,
            "unit": "µs",
            "extra": "min 285.1 µs, p95 565.5 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 516.8,
            "unit": "µs",
            "extra": "min 467.3 µs, p95 682 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33950.1,
            "unit": "µs",
            "extra": "min 33495.9 µs, p95 34907.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34948.9,
            "unit": "µs",
            "extra": "min 34329.3 µs, p95 35648.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41791.7,
            "unit": "µs",
            "extra": "min 41377.6 µs, p95 42129.7 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 284.4,
            "unit": "µs",
            "extra": "min 274.8 µs, p95 319 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 340.9,
            "unit": "µs",
            "extra": "min 327.4 µs, p95 371.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 405.8,
            "unit": "µs",
            "extra": "min 392.1 µs, p95 429.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.22,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.57,
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
            "value": 0.204,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 11.2,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.854,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 28.12,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 29.86,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.35,
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
          "id": "e3cf39ba68bc60027de7f29f0c8f1df749b857dc",
          "message": "feat: S1.6 text and binary bodies verify, through a text component and bytes\n\nADR 0036. A v1–v4 pact whose body was plain text failed as\ncomponent-unavailable, and a binary one as contract-invalid; today's verifier\npasses both.\n\n- engine/component-text is a built-in content component for text/*: it decodes\n  UTF-8 octets to a string and encodes a string back, whatever the charset\n  says. Every embedding registers it after JSON, and engine/hello lists it.\n- A content slot no component claims now decodes to bytes rather than absent.\n  One a component claims and cannot decode is still absent.\n- The legacy layer reads a body by the pact's content type, as today's\n  verifier picks a body matcher: JSON as a document, a binary type as bytes,\n  other text as a string (bytes when not UTF-8), and XML, form and multipart\n  refused until S1.21 to S1.23. A text or bytes body compiles to one value at\n  $: its rules, or match:equality.\n- A reply's body is decoded as the pact's type, and a new legacy action,\n  legacy:body-type, fails a reply whose content type disagrees, with today's\n  message. It is emitted only when the pact names no Content-Type header, so\n  six legacy corpus snapshots show it now; their results are unchanged.\n- A bytes mismatch names the first byte that differs.\n- A corpus case may carry bytes through an optional `encoded` map\n  (plan-grammar spec §6.1). New corpus cases cover text, regex text and\n  binary bodies; corpora/verifier/bodies/ adds the harness fixtures, and the\n  four S1.6 entries leave the disagreement ledger.\n\nThe plan gains S1.25 (janus upgrade converts text and binary bodies) and S6.5\n(charsets other than UTF-8, in the text component).\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T15:47:40+11:00",
          "tree_id": "11561b2d312220e3177a2f2511ddd861e90f39f4",
          "url": "https://github.com/rholshausen/pact-janus/commit/e3cf39ba68bc60027de7f29f0c8f1df749b857dc"
        },
        "date": 1792558925244,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 14.6,
            "unit": "µs",
            "extra": "min 8.8 µs, p95 19.2 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2232.4,
            "unit": "µs",
            "extra": "min 2155.6 µs, p95 2313.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 217.7,
            "unit": "µs",
            "extra": "min 178.3 µs, p95 290.3 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2545.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7653830,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 35766.7,
            "unit": "µs",
            "extra": "min 34870.3 µs, p95 36920.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 39143.8,
            "unit": "µs",
            "extra": "min 37992.9 µs, p95 40541.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 42366.7,
            "unit": "µs",
            "extra": "min 41801.5 µs, p95 42906.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 435.6,
            "unit": "µs",
            "extra": "min 414.9 µs, p95 462.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 516.3,
            "unit": "µs",
            "extra": "min 470.3 µs, p95 583.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 494.4,
            "unit": "µs",
            "extra": "min 470.1 µs, p95 519.9 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18831,
            "unit": "µs",
            "extra": "min 16641.6 µs, p95 20029.4 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 20172.9,
            "unit": "µs",
            "extra": "min 19242.5 µs, p95 20716.4 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 18.9,
            "unit": "µs",
            "extra": "min 18.5 µs, p95 19.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.9,
            "unit": "µs",
            "extra": "min 25.5 µs, p95 26.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 27464.9,
            "unit": "µs",
            "extra": "min 25399.1 µs, p95 29027.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 27072.4,
            "unit": "µs",
            "extra": "min 25369.4 µs, p95 32658.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 15506.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 15366.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 142.9,
            "unit": "µs",
            "extra": "min 129.7 µs, p95 155.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 179.6,
            "unit": "µs",
            "extra": "min 168.6 µs, p95 194.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 26.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 66.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 135.9,
            "unit": "µs",
            "extra": "min 117.2 µs, p95 147.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 173,
            "unit": "µs",
            "extra": "min 143.8 µs, p95 190.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 23.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 59.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 380.3,
            "unit": "µs",
            "extra": "min 296.5 µs, p95 511.8 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 505.6,
            "unit": "µs",
            "extra": "min 457.5 µs, p95 741.8 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 34506.3,
            "unit": "µs",
            "extra": "min 33585.7 µs, p95 35262.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 37721.3,
            "unit": "µs",
            "extra": "min 35569.1 µs, p95 38998.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 42187.3,
            "unit": "µs",
            "extra": "min 41655.4 µs, p95 43412.2 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 284,
            "unit": "µs",
            "extra": "min 274.3 µs, p95 311.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 399.5,
            "unit": "µs",
            "extra": "min 355.5 µs, p95 443.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 411.6,
            "unit": "µs",
            "extra": "min 396.3 µs, p95 436.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.19,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.7,
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
            "value": 0.177,
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
            "value": 11.9,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.857,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 29.6,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 30.48,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.35,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.38,
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
          "id": "a862fdb4818e876cdadc8ec08a8229606af26d83",
          "message": "fix: a bytes value in a plan shows its length and base64\n\nThe text forms printed a bytes value as `<8 byte(s)>`, which said nothing\nabout the bytes, and `<...>` already means `<absent>` in mismatch messages.\nThey now print `BYTES(8, iVBORw0KGgo=)`, as the forked pact_matching engine\ndid: the length, then the base64, which is how a v1–v4 pact and the protocol\ncarry the bytes. The base64 is cut after 48 characters (the first 36 octets,\non a 4-character boundary) and ends in `...`, which no base64 alphabet holds.\n\nOne function in plan/render.rs prints bytes for the pretty and executed forms\nand for mismatch messages; the highlighter's lexer reads the new form as a\nliteral. Plan-grammar spec §3.1 states the form, and the two binary-body\ncorpus cases are re-accepted with it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T17:20:49+11:00",
          "tree_id": "649d863fc9927a70be678ebb688be341f1e08ce0",
          "url": "https://github.com/rholshausen/pact-janus/commit/a862fdb4818e876cdadc8ec08a8229606af26d83"
        },
        "date": 1792563929293,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 14.6,
            "unit": "µs",
            "extra": "min 8.9 µs, p95 19 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2217.2,
            "unit": "µs",
            "extra": "min 2168.1 µs, p95 2380.1 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 190.7,
            "unit": "µs",
            "extra": "min 173.3 µs, p95 214.8 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2488.7,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7658309,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 34795.2,
            "unit": "µs",
            "extra": "min 34188.7 µs, p95 35279.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 37288.8,
            "unit": "µs",
            "extra": "min 36655 µs, p95 39798.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41502.5,
            "unit": "µs",
            "extra": "min 41181.3 µs, p95 42062.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 430.9,
            "unit": "µs",
            "extra": "min 413.6 µs, p95 452.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 510.5,
            "unit": "µs",
            "extra": "min 458.3 µs, p95 558.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 487,
            "unit": "µs",
            "extra": "min 463 µs, p95 508 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 16862.5,
            "unit": "µs",
            "extra": "min 16331.2 µs, p95 17359.8 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 18640.9,
            "unit": "µs",
            "extra": "min 18393.3 µs, p95 18893.4 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.4,
            "unit": "µs",
            "extra": "min 18.9 µs, p95 20.1 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 26.2,
            "unit": "µs",
            "extra": "min 25.8 µs, p95 27 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 23747.8,
            "unit": "µs",
            "extra": "min 23049.7 µs, p95 24377.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 24913.6,
            "unit": "µs",
            "extra": "min 22997.5 µs, p95 26274.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13086.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13377.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 116.3,
            "unit": "µs",
            "extra": "min 102.9 µs, p95 145.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 137.2,
            "unit": "µs",
            "extra": "min 130.3 µs, p95 178 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 22.4,
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
            "value": 114.1,
            "unit": "µs",
            "extra": "min 100.3 µs, p95 146 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 132.6,
            "unit": "µs",
            "extra": "min 125.9 µs, p95 169.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 25.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 49.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 366.4,
            "unit": "µs",
            "extra": "min 273.8 µs, p95 572.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 464.4,
            "unit": "µs",
            "extra": "min 404.5 µs, p95 658 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33513.2,
            "unit": "µs",
            "extra": "min 33131.5 µs, p95 33994.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 35134.2,
            "unit": "µs",
            "extra": "min 34432.1 µs, p95 37243.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41488.3,
            "unit": "µs",
            "extra": "min 41050.9 µs, p95 41873.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 280.5,
            "unit": "µs",
            "extra": "min 271.4 µs, p95 312.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 376.4,
            "unit": "µs",
            "extra": "min 365.1 µs, p95 411 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 404.7,
            "unit": "µs",
            "extra": "min 386.3 µs, p95 433.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.14,
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
            "value": 0.34,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.176,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.194,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 10.88,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.846,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 28.44,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 29.61,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.34,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.37,
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
            "value": 0.5,
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
          "id": "816e4b7c8c22e9e0b0412419ac6ef182cff22dc6",
          "message": "fix: the built-in text component serves only a body that is a string\n\nS1.6's text component claims text/*, so a text/csv body of rows with no CSV\ncomponent declared stopped failing as component-unavailable: the content-type\ncheck found text, and the mock would only have failed later, with\nencode-failed, when it tried to write the rows. The TypeScript SDK's CSV\nconsumer test caught it in CI.\n\nThe text component's compile now refuses a shape that admits no string\n(objects, arrays, each-like, numbers, booleans, and the choices and wrappers\nof only those). The engine asks the in-tree component that handles a declared\nslot to compile its shape when the interaction arrives and before a\nverification starts, as it already asked declared components, and a failure\nis component-unavailable with reason compile-failed, naming content/<type>.\nScope::check now runs that and the declared components' fragment check\ntogether. A text/ body that is one string is still served as text with no\ncomponent declared; v1–v4 pacts do not go through this check.\n\nADR 0036 decision 1 and component-interfaces spec §6.3 say so, and the\nunreleased S1.6 change file is updated to match.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T17:34:07+11:00",
          "tree_id": "02f2865b3390ce4c44e7589a77ff22af9ae11e08",
          "url": "https://github.com/rholshausen/pact-janus/commit/816e4b7c8c22e9e0b0412419ac6ef182cff22dc6"
        },
        "date": 1792565283946,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 14,
            "unit": "µs",
            "extra": "min 8.4 µs, p95 30.5 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1720.4,
            "unit": "µs",
            "extra": "min 1606.1 µs, p95 3486.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 223.3,
            "unit": "µs",
            "extra": "min 176.2 µs, p95 266.3 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2488.5,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7666650,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 37471.3,
            "unit": "µs",
            "extra": "min 36101.1 µs, p95 40134.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 39531,
            "unit": "µs",
            "extra": "min 38386.2 µs, p95 41730.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 44349.8,
            "unit": "µs",
            "extra": "min 43574.8 µs, p95 45094 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 442.7,
            "unit": "µs",
            "extra": "min 422.1 µs, p95 459.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 503.3,
            "unit": "µs",
            "extra": "min 450.1 µs, p95 560.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 513.2,
            "unit": "µs",
            "extra": "min 489.2 µs, p95 529.5 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 19087.8,
            "unit": "µs",
            "extra": "min 17735.6 µs, p95 20198.2 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19107.2,
            "unit": "µs",
            "extra": "min 18272.4 µs, p95 19655.3 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 18.9,
            "unit": "µs",
            "extra": "min 18.5 µs, p95 19.2 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 25.7,
            "unit": "µs",
            "extra": "min 24.9 µs, p95 26.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 30321.3,
            "unit": "µs",
            "extra": "min 27480 µs, p95 32868.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 30192.9,
            "unit": "µs",
            "extra": "min 27996.5 µs, p95 36533.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 18097.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 17787.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 111.4,
            "unit": "µs",
            "extra": "min 104.1 µs, p95 126.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 126.8,
            "unit": "µs",
            "extra": "min 119.1 µs, p95 142.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 24.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 46.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 100.6,
            "unit": "µs",
            "extra": "min 87.6 µs, p95 121.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 125.2,
            "unit": "µs",
            "extra": "min 103.5 µs, p95 159.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 19.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 48.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 338.2,
            "unit": "µs",
            "extra": "min 227.5 µs, p95 457.8 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 397,
            "unit": "µs",
            "extra": "min 306.6 µs, p95 703.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 35182.8,
            "unit": "µs",
            "extra": "min 33975.6 µs, p95 37421.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 37766,
            "unit": "µs",
            "extra": "min 35774 µs, p95 40656.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 43949.6,
            "unit": "µs",
            "extra": "min 43282.4 µs, p95 44918.7 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 292.1,
            "unit": "µs",
            "extra": "min 278.3 µs, p95 306.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 348,
            "unit": "µs",
            "extra": "min 327 µs, p95 387.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 402.6,
            "unit": "µs",
            "extra": "min 384.1 µs, p95 423.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.97,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.29,
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
            "value": 0.163,
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
            "value": 9.61,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.844,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 30.95,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 30.81,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.28,
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
            "value": 0.47,
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
          "id": "8a8449e082b0e74549f10c83899393fdd89d8773",
          "message": "feat: S1.20 member order is carried through the engine, and decided by nothing\n\nADR 0037. The engine sorted every object's members, so a mock served a CSV\nbody's columns, or a JSON body's members, alphabetically whatever order the\nconsumer declared them in (phase-9 finding 11).\n\n- serde_json's preserve_order is on, the runtime value's objects and an\n  object shape's members are insertion-ordered, and a generated document\n  follows the declared order, so content/encode sees the author's order.\n- Order is never compared: objects with the same members are equal in any\n  order, and plans, the variant space, subsumption and the v1-v4 compiler\n  walk members by name. Two places had begun reading order once it was\n  kept and now go by name: subsumption's same-admits test, which zipped two\n  members maps, and janus upgrade's each-entry sample.\n- The canonical writer had relied on sorted maps. Records and a shape's own\n  members are written by name; an object shape's members, examples, options,\n  recorded content and state parameter values keep their order\n  (contract-file spec §2.4). Provider shapes follow the same rule. A contract\n  whose members were in name order writes the same bytes as before.\n- The legacy compiler compiles a body as a document (LegacyBody::Document of\n  a RuntimeValue), not serde_json::Value (kernel-boundary review finding 5),\n  so form and XML components can hand it what they decode.\n- ComponentError and EngineError box their details, since an ordered Value\n  is larger; the protocol is unchanged.\n\nNo plan or result in corpora/ moved; five executed.txt snapshots print a\ncaptured body in the order the case wrote it. A new case,\nshapes/member-order, compares an example and a body holding the same members\nin other orders. S1.1's harness: 326 verdicts, 312 agree, 14 known. The CSV\nhost test and the TypeScript SDK's CSV consumer check the column order.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T11:07:20+11:00",
          "tree_id": "e0953a1a18802820d0a531e8f1af65fe4ae8e62a",
          "url": "https://github.com/rholshausen/pact-janus/commit/8a8449e082b0e74549f10c83899393fdd89d8773"
        },
        "date": 1792627973517,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 11.4,
            "unit": "µs",
            "extra": "min 7.7 µs, p95 16.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1893.8,
            "unit": "µs",
            "extra": "min 1797.4 µs, p95 2002.1 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 145.2,
            "unit": "µs",
            "extra": "min 105.5 µs, p95 162.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1873.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7604902,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 38503.6,
            "unit": "µs",
            "extra": "min 36901.1 µs, p95 40795.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 39826.5,
            "unit": "µs",
            "extra": "min 38320.2 µs, p95 40752.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 43491.8,
            "unit": "µs",
            "extra": "min 42569.3 µs, p95 44698.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 400.3,
            "unit": "µs",
            "extra": "min 388.3 µs, p95 414.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 412.2,
            "unit": "µs",
            "extra": "min 398.6 µs, p95 431.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 429.3,
            "unit": "µs",
            "extra": "min 401 µs, p95 462.2 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 17680.6,
            "unit": "µs",
            "extra": "min 15863.7 µs, p95 19686.2 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 16643.7,
            "unit": "µs",
            "extra": "min 16295.8 µs, p95 17647 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.7,
            "unit": "µs",
            "extra": "min 17.8 µs, p95 20.2 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 22.1,
            "unit": "µs",
            "extra": "min 21.5 µs, p95 22.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 27007.2,
            "unit": "µs",
            "extra": "min 25167.1 µs, p95 29206 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 25689.6,
            "unit": "µs",
            "extra": "min 23024 µs, p95 27710.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 16209.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 15494.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 86,
            "unit": "µs",
            "extra": "min 72.4 µs, p95 104.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 100.9,
            "unit": "µs",
            "extra": "min 95.8 µs, p95 127.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 19.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 39.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 73.8,
            "unit": "µs",
            "extra": "min 68.3 µs, p95 99.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 100.5,
            "unit": "µs",
            "extra": "min 86 µs, p95 125.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 16.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 37.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 293.4,
            "unit": "µs",
            "extra": "min 249.1 µs, p95 347.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 375.3,
            "unit": "µs",
            "extra": "min 334.3 µs, p95 497.3 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 35426.5,
            "unit": "µs",
            "extra": "min 34536.2 µs, p95 36789.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 37421.3,
            "unit": "µs",
            "extra": "min 36543.2 µs, p95 38616.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 43417.3,
            "unit": "µs",
            "extra": "min 42404.8 µs, p95 45764.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 235.2,
            "unit": "µs",
            "extra": "min 227.3 µs, p95 254.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 277.9,
            "unit": "µs",
            "extra": "min 267.5 µs, p95 310.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 336.7,
            "unit": "µs",
            "extra": "min 325.1 µs, p95 366 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.75,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.01,
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
            "value": 0.32,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.138,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.156,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 9.14,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.66,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 31.08,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 32.13,
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
          "id": "648b4dc90df51ab7cfa44e8c19f0290507d98c59",
          "message": "fix: the broker client's recording keeps the order the broker sends\n\nCI's verifier-diff job records the broker scenario again and fails on any\ndifference. Since S1.20 (ADR 0037) a Value keeps its members in the order\nthey arrived, so the recorder writes the broker's replies and our requests\nin their own order rather than sorted. The content is unchanged, the 409 the\nscenario provokes included; only member order moved. Recorded again against\nthe pinned broker, and the result is byte-identical across two fresh\nbrokers.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T12:06:08+11:00",
          "tree_id": "2d7e6b70ea0f8e75ea828f14660cfb394445ea07",
          "url": "https://github.com/rholshausen/pact-janus/commit/648b4dc90df51ab7cfa44e8c19f0290507d98c59"
        },
        "date": 1792631508544,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 9.9,
            "unit": "µs",
            "extra": "min 9.4 µs, p95 14.8 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2145,
            "unit": "µs",
            "extra": "min 2082.4 µs, p95 2213.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 178.1,
            "unit": "µs",
            "extra": "min 148.2 µs, p95 552.1 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2484.2,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7710528,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 41900.6,
            "unit": "µs",
            "extra": "min 41097.7 µs, p95 42987.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 42619.4,
            "unit": "µs",
            "extra": "min 41703.2 µs, p95 43442.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 50987.3,
            "unit": "µs",
            "extra": "min 50661.7 µs, p95 51493.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 495.4,
            "unit": "µs",
            "extra": "min 474.9 µs, p95 512 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 582.2,
            "unit": "µs",
            "extra": "min 510.9 µs, p95 603 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 585.2,
            "unit": "µs",
            "extra": "min 556.8 µs, p95 609.2 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18573.1,
            "unit": "µs",
            "extra": "min 18135.1 µs, p95 19329.6 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 20939.7,
            "unit": "µs",
            "extra": "min 20682.3 µs, p95 21328.9 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 22.6,
            "unit": "µs",
            "extra": "min 21.9 µs, p95 23.4 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 28,
            "unit": "µs",
            "extra": "min 27.6 µs, p95 28.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 26340.2,
            "unit": "µs",
            "extra": "min 25537 µs, p95 28333.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 26955.8,
            "unit": "µs",
            "extra": "min 26050.5 µs, p95 29870.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 14898.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 15435.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 134.2,
            "unit": "µs",
            "extra": "min 124.3 µs, p95 141.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 174.1,
            "unit": "µs",
            "extra": "min 155.2 µs, p95 183.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 24.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 60,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 129.7,
            "unit": "µs",
            "extra": "min 107.1 µs, p95 142.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 172.6,
            "unit": "µs",
            "extra": "min 146.3 µs, p95 182.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 24,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 66.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 386.5,
            "unit": "µs",
            "extra": "min 313.2 µs, p95 539.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 476.4,
            "unit": "µs",
            "extra": "min 410.6 µs, p95 634 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 38762.5,
            "unit": "µs",
            "extra": "min 38425.9 µs, p95 39287.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 40041.8,
            "unit": "µs",
            "extra": "min 39425.6 µs, p95 42363 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 50688.5,
            "unit": "µs",
            "extra": "min 50355.5 µs, p95 51112.8 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 305.6,
            "unit": "µs",
            "extra": "min 296.8 µs, p95 330.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 392.7,
            "unit": "µs",
            "extra": "min 368.9 µs, p95 492.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 453.4,
            "unit": "µs",
            "extra": "min 435.4 µs, p95 475.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.28,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.65,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.36,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.4,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.192,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.215,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 11.29,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.829,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 32.6,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 33.65,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.36,
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
            "value": 0.49,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "8aaf6091a572b0a41bc17a7d49e8ca733377f414",
          "message": "docs: cite pact-reference#588 for today's unapplied form body generators\n\nThe ledger's two generators/form-body entries name the upstream issue:\npact_verifier builds pact_matching without its form_urlencoded feature, so\ntoday's verifier sends a form body's generators unapplied. ADR 0038 said\ntoday's verifier generates into a form body; it now says pact_matching does\nwhen built to, pact_verifier is not, and pact-jvm does, citing the issue.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T12:15:33+11:00",
          "tree_id": "a3a786f54eb917e612cdceb90f07f528e2d22b31",
          "url": "https://github.com/rholshausen/pact-janus/commit/8aaf6091a572b0a41bc17a7d49e8ca733377f414"
        },
        "date": 1792632067460,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8,
            "unit": "µs",
            "extra": "min 7.6 µs, p95 11.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1875.7,
            "unit": "µs",
            "extra": "min 1833.5 µs, p95 1944 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 168,
            "unit": "µs",
            "extra": "min 146.4 µs, p95 660.1 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1862.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7710528,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 38126.2,
            "unit": "µs",
            "extra": "min 36885.2 µs, p95 39410.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 36578.6,
            "unit": "µs",
            "extra": "min 35858.6 µs, p95 38063.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41337.4,
            "unit": "µs",
            "extra": "min 40918.8 µs, p95 41626.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 399.9,
            "unit": "µs",
            "extra": "min 386 µs, p95 414 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 452,
            "unit": "µs",
            "extra": "min 397 µs, p95 467.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 424.5,
            "unit": "µs",
            "extra": "min 400.4 µs, p95 450.1 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 15723.2,
            "unit": "µs",
            "extra": "min 15289.9 µs, p95 15938.7 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 16413.2,
            "unit": "µs",
            "extra": "min 16162.4 µs, p95 16655.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 18.6,
            "unit": "µs",
            "extra": "min 17.2 µs, p95 19.1 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 22.1,
            "unit": "µs",
            "extra": "min 21.6 µs, p95 22.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 22672.1,
            "unit": "µs",
            "extra": "min 21894.3 µs, p95 23177.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 23042.7,
            "unit": "µs",
            "extra": "min 22262.5 µs, p95 25376.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 13392.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13695.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 83.6,
            "unit": "µs",
            "extra": "min 74 µs, p95 100.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 110.7,
            "unit": "µs",
            "extra": "min 98 µs, p95 129.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 16.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 41.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 76.7,
            "unit": "µs",
            "extra": "min 69.8 µs, p95 90.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 106.2,
            "unit": "µs",
            "extra": "min 93.2 µs, p95 120 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 15.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 44.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 286.2,
            "unit": "µs",
            "extra": "min 252.9 µs, p95 397.2 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 373.7,
            "unit": "µs",
            "extra": "min 329.8 µs, p95 496.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 35679.5,
            "unit": "µs",
            "extra": "min 34512.8 µs, p95 36118.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 34666.2,
            "unit": "µs",
            "extra": "min 33665.8 µs, p95 35394.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 40980.5,
            "unit": "µs",
            "extra": "min 40629.8 µs, p95 41187.9 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 231.6,
            "unit": "µs",
            "extra": "min 221.3 µs, p95 245.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 281.5,
            "unit": "µs",
            "extra": "min 273.3 µs, p95 301.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 334.7,
            "unit": "µs",
            "extra": "min 324.2 µs, p95 351.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.75,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.98,
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
            "value": 0.32,
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
            "value": 0.159,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 9.26,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.662,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 27.49,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 27.61,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.28,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.29,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.39,
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
          "id": "1dbc1d278e2ed5d00f929cdd3546194cf819594e",
          "message": "chore: the benchmark chart draws a trend line, the median of the last runs\n\nThe runners are shared machines, so a single run's numbers move by tens of\npercent and a change in the code is hard to see on the chart. The chart page\nnow draws, at each run, the median of the last 7 runs over the runs\nthemselves, which are faded; ?window=N on the address changes how many.\n\nThe page was github-action-benchmark's default, which it writes to gh-pages\nonly when there is none. Its source now lives in benchmarks/chart/index.html,\nand the Benchmarks workflow publishes it as bench/index.html after each\nrecorded run, committing only when it differs.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T12:40:35+11:00",
          "tree_id": "9956137246a351b5bf1dd3dacb31d6a210472a63",
          "url": "https://github.com/rholshausen/pact-janus/commit/1dbc1d278e2ed5d00f929cdd3546194cf819594e"
        },
        "date": 1792633535877,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 9.9,
            "unit": "µs",
            "extra": "min 9.5 µs, p95 16 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2188.6,
            "unit": "µs",
            "extra": "min 2123.7 µs, p95 2296.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 235.1,
            "unit": "µs",
            "extra": "min 194.8 µs, p95 391.8 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2493.2,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7710528,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 44879.3,
            "unit": "µs",
            "extra": "min 42175.2 µs, p95 47039.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 44870.9,
            "unit": "µs",
            "extra": "min 42515.8 µs, p95 46697.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 51894.5,
            "unit": "µs",
            "extra": "min 51045.4 µs, p95 53023 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 500.7,
            "unit": "µs",
            "extra": "min 479.4 µs, p95 520.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 574.4,
            "unit": "µs",
            "extra": "min 521.6 µs, p95 608.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 583.5,
            "unit": "µs",
            "extra": "min 558.9 µs, p95 612.9 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 20550.9,
            "unit": "µs",
            "extra": "min 18834.2 µs, p95 21894.8 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 21777.3,
            "unit": "µs",
            "extra": "min 20869.4 µs, p95 22845 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 23.2,
            "unit": "µs",
            "extra": "min 21.4 µs, p95 24.2 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 28.2,
            "unit": "µs",
            "extra": "min 27.8 µs, p95 29.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 30053.9,
            "unit": "µs",
            "extra": "min 26681.2 µs, p95 33420.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 29399.5,
            "unit": "µs",
            "extra": "min 27304 µs, p95 35943.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 17625.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 17219.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 144.2,
            "unit": "µs",
            "extra": "min 127.9 µs, p95 156.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 188.9,
            "unit": "µs",
            "extra": "min 161.2 µs, p95 214.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 26.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 70.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 143.1,
            "unit": "µs",
            "extra": "min 125.8 µs, p95 155.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 186.8,
            "unit": "µs",
            "extra": "min 156.9 µs, p95 202.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 29,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 75.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 413.7,
            "unit": "µs",
            "extra": "min 309.7 µs, p95 471.7 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 543.1,
            "unit": "µs",
            "extra": "min 449 µs, p95 755.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 40755.1,
            "unit": "µs",
            "extra": "min 38926.2 µs, p95 42218.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 41763.9,
            "unit": "µs",
            "extra": "min 40134.6 µs, p95 43669.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 52204.3,
            "unit": "µs",
            "extra": "min 51084.4 µs, p95 53002.2 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 314.6,
            "unit": "µs",
            "extra": "min 300.7 µs, p95 336.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 458.3,
            "unit": "µs",
            "extra": "min 390.7 µs, p95 482.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 453.1,
            "unit": "µs",
            "extra": "min 434.7 µs, p95 483.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.39,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.76,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.37,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.41,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.198,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.224,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 11.92,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.864,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 34.83,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 35.98,
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
            "value": 0.41,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "bc39a46924ad38f93319780dbc31296e1b10da52",
          "message": "feat: S1.22 XML bodies verify as today, through a built-in XML component\n\nA v1–v4 pact with an XML body was refused, and the legacy harness skipped\nthe 156 pact-specification cases that have one. ADR 0039 decides how XML\nis read and compared.\n\nengine/component-xml is a built-in content component for the types today's\nverifier counts as XML (application or text, subtype xml or +xml). Every\nembedding registers it after JSON and before text, so text/xml reaches it.\nIt parses with sxd-document, as today's verifier does. Its document is\nkeyed by element name: an element holds #namespace, @<name> per attribute,\n#text (each text node trimmed, then joined) and an array of children per\nlocal name, so pact's XML paths are document paths.\n\nThe legacy layer reads a pact's XML body into the same document, and a test\nholds that reader to the component's. The legacy compiler walks it as\npact_matching's match_xml does, with three new legacy actions:\nlegacy:xml-children, legacy:xml-only-children and legacy:parses-as. Plan\npaths gain a quoted segment, ['@{urn:names}name']. Request generators on an\nXML body go through pact_models' XmlHandler.\n\nA type rule on any v1–v4 text value (attribute, element text, path, header,\nquery or form value) now parses the text, as today's verifier does, where\nit compared the value's kind.\n\nThe legacy harness runs all 741 cases, with no exclusion rule and no\njanus:skip. The differential harness reads 344 verdicts: 330 agree and 14\nare known. Its ledger entries for compatibility-suite case 19 are gone. One\nnew entry is deliberate: an element the pact puts in no namespace matches\nits local name in any namespace, as the specification's `unexpected xml\nnamespace` case says and pact-reference's V2 engine does, where today's\nverifier and pact-jvm fail it.\n\nThe staged plan adds S2.10: keeping XML text as written, CDATA included,\nin Janus contracts only.\n\nThe XML walk follows pact_matching::xml (pact-reference, rust/pact_matching\n2.0.11); no code was copied.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T16:18:29+11:00",
          "tree_id": "d6d6727384e3368ffa58a5d037dd75287f00123d",
          "url": "https://github.com/rholshausen/pact-janus/commit/bc39a46924ad38f93319780dbc31296e1b10da52"
        },
        "date": 1792646613306,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10.2,
            "unit": "µs",
            "extra": "min 9.7 µs, p95 17.5 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2185.3,
            "unit": "µs",
            "extra": "min 2109.2 µs, p95 2258.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 194.9,
            "unit": "µs",
            "extra": "min 172.9 µs, p95 251.1 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2555.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7999326,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 43611,
            "unit": "µs",
            "extra": "min 41719.7 µs, p95 45244.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 43973.4,
            "unit": "µs",
            "extra": "min 42511.5 µs, p95 45994.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 51031.6,
            "unit": "µs",
            "extra": "min 50534 µs, p95 52010.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 502.9,
            "unit": "µs",
            "extra": "min 484.8 µs, p95 518.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 553.3,
            "unit": "µs",
            "extra": "min 521 µs, p95 602.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 587.2,
            "unit": "µs",
            "extra": "min 561.8 µs, p95 609.4 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 19613.4,
            "unit": "µs",
            "extra": "min 18167.3 µs, p95 21953.8 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 21187.6,
            "unit": "µs",
            "extra": "min 20971 µs, p95 22106.4 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 22.7,
            "unit": "µs",
            "extra": "min 20.8 µs, p95 23.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 28.6,
            "unit": "µs",
            "extra": "min 28.2 µs, p95 29.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 30470.4,
            "unit": "µs",
            "extra": "min 27557.9 µs, p95 33439.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 28999.6,
            "unit": "µs",
            "extra": "min 26517.1 µs, p95 36399.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 17778.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 16641,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 151.9,
            "unit": "µs",
            "extra": "min 141.2 µs, p95 159.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 182.6,
            "unit": "µs",
            "extra": "min 158.9 µs, p95 203.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 30.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 68.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 143.1,
            "unit": "µs",
            "extra": "min 124.9 µs, p95 160.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 182.2,
            "unit": "µs",
            "extra": "min 149.7 µs, p95 199 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 29.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 69,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 430.7,
            "unit": "µs",
            "extra": "min 331.6 µs, p95 764.7 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 530.6,
            "unit": "µs",
            "extra": "min 433 µs, p95 581.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 39819.8,
            "unit": "µs",
            "extra": "min 38894.4 µs, p95 41824.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 40862.4,
            "unit": "µs",
            "extra": "min 40073 µs, p95 42375.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 51211.4,
            "unit": "µs",
            "extra": "min 50569.1 µs, p95 52016 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 306.8,
            "unit": "µs",
            "extra": "min 295.1 µs, p95 328.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 458.5,
            "unit": "µs",
            "extra": "min 389.8 µs, p95 488.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 451.5,
            "unit": "µs",
            "extra": "min 436.6 µs, p95 470.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.36,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.75,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.38,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.41,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.198,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.226,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 11.91,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.87,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 35.41,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 35.81,
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
            "value": 0.42,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "f2da18e712b02bdcf2cc6df6623284295781fb5f",
          "message": "docs: the v1–v4 compiler may emit expect:absent for a body declared empty\n\nPlan-grammar §4.4 and §4.5 said the v1–v4 compiler never emits\nexpect:absent, since v1–v4 has no absence assertion. It has one: a body\nthe pact declares empty or null fails when the actual body has content\n(\"Expected empty body but received …\", pact_matching 2.0.11\nmatch_body_content) and passes when it is missing or empty. The legacy\ncompiler has emitted expect:absent on the body slot for exactly that since\nthe prototype, and the specification case `body/non empty body found when\nempty expected` holds it to today's verdict.\n\nThe spec now allows expect:absent there and nowhere else in a v1–v4 plan;\nmatch:any-of stays shapes only. No behaviour or corpus changes. Found while\nlanding S1.22.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T16:22:57+11:00",
          "tree_id": "25a0f07018c6aa287d7a56a7cdf90f2d94948293",
          "url": "https://github.com/rholshausen/pact-janus/commit/f2da18e712b02bdcf2cc6df6623284295781fb5f"
        },
        "date": 1792646940502,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10.1,
            "unit": "µs",
            "extra": "min 9.9 µs, p95 15.1 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2101.7,
            "unit": "µs",
            "extra": "min 2059 µs, p95 2173.1 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 211.7,
            "unit": "µs",
            "extra": "min 165 µs, p95 277 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2557.9,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 7999326,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 41483.5,
            "unit": "µs",
            "extra": "min 41127.6 µs, p95 42358.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 43044.3,
            "unit": "µs",
            "extra": "min 42122 µs, p95 44808.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 51020.1,
            "unit": "µs",
            "extra": "min 50570.3 µs, p95 52024.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 505.8,
            "unit": "µs",
            "extra": "min 485.6 µs, p95 520.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 568.1,
            "unit": "µs",
            "extra": "min 521 µs, p95 604.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 579.9,
            "unit": "µs",
            "extra": "min 555.9 µs, p95 600.8 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18577.2,
            "unit": "µs",
            "extra": "min 18343.5 µs, p95 18880.2 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 21257.1,
            "unit": "µs",
            "extra": "min 20945.3 µs, p95 22494.4 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 22.9,
            "unit": "µs",
            "extra": "min 21.6 µs, p95 23.7 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 28.5,
            "unit": "µs",
            "extra": "min 28.2 µs, p95 31.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 26242,
            "unit": "µs",
            "extra": "min 25394.7 µs, p95 27338.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 27318.7,
            "unit": "µs",
            "extra": "min 26251.3 µs, p95 32823.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 14665.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 15327,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 141.9,
            "unit": "µs",
            "extra": "min 131.9 µs, p95 148.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 187.9,
            "unit": "µs",
            "extra": "min 162.5 µs, p95 194.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 25.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 71.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 144.4,
            "unit": "µs",
            "extra": "min 127.9 µs, p95 149 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 181.6,
            "unit": "µs",
            "extra": "min 155.2 µs, p95 199.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 29.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 68.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 405.8,
            "unit": "µs",
            "extra": "min 333.4 µs, p95 534.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 489.5,
            "unit": "µs",
            "extra": "min 412.9 µs, p95 650.3 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 38625.9,
            "unit": "µs",
            "extra": "min 38194.5 µs, p95 38996.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 40418.1,
            "unit": "µs",
            "extra": "min 39746.9 µs, p95 41540 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 51010.5,
            "unit": "µs",
            "extra": "min 50542.4 µs, p95 52055.2 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 310.2,
            "unit": "µs",
            "extra": "min 301.1 µs, p95 331.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 416.1,
            "unit": "µs",
            "extra": "min 387 µs, p95 477.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 456.8,
            "unit": "µs",
            "extra": "min 436.7 µs, p95 514.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.34,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.73,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.37,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.4,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.195,
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
            "value": 11.37,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.836,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 32.4,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 33.96,
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
            "value": 0.5,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "ba66863adea760556b544dbea6229fd262edb70e",
          "message": "feat: S1.23 multipart bodies verify as today, through a built-in multipart component\n\nA v1–v4 pact with a multipart body was refused as component-unavailable, the\nlast body kind the legacy layer refused. ADR 0040 decides how multipart is\nread and compared.\n\nengine/component-multipart claims every multipart/* type and is registered\nafter form in every embedding. It splits a body with multer, as today's\nverifier does, at the boundary its content type names (read with mime, as\ntoday; multer's own parse_boundary reads only multipart/form-data). Its\ndocument is keyed by part name, unnamed parts under \"\", and a part is\nheaders, filename and body. Each part's body is decoded through the built-in\nJSON, XML, text and form components the multipart component holds itself;\nany other part is bytes. Reaching declared components is the new S5.6.\n\nThe legacy layer splits a pact's body the same way and reads each part by\nits own type, and a test holds that reader to the component's over every\nmultipart body in the repository. The legacy compiler compares the parts as\npact_matching's match_mime_multipart does: by name, or by position among the\nunnamed parts; a new legacy action, legacy:mime-part, fails a missing part\nor the wrong kind with today's messages; then each part's headers, and its\nbody, a text field as text and a file by its own type with the rules under\n$.<part> re-rooted. A reply is decoded under its own label, because its\nboundary is its own. Unsupported::ContentType is gone: no body kind is\nrefused.\n\nThe differential harness reads 399 verdicts over 183 cases: 389 agree and 10\nare known. The four ledger entries for compatibility-suite case 25 are gone.\nbodies/every-body-kind, one v3 pact with XML, form, multipart and binary\nbodies (S1.6's combined exit), and bodies/multipart-form-data agree with\ntoday's verifier, mutations included.\n\nThe multipart walk follows pact_matching::binary_utils (pact-reference,\nrust/pact_matching 2.0.11); no code was copied.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T17:39:22+11:00",
          "tree_id": "17de75f266163bbf0117d8fe9da1636793d70f63",
          "url": "https://github.com/rholshausen/pact-janus/commit/ba66863adea760556b544dbea6229fd262edb70e"
        },
        "date": 1792651421828,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 5.5,
            "unit": "µs",
            "extra": "min 5.3 µs, p95 10.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1807.1,
            "unit": "µs",
            "extra": "min 1743.4 µs, p95 1899.6 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 106.3,
            "unit": "µs",
            "extra": "min 102.2 µs, p95 141.9 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1571.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8189668,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 30376.6,
            "unit": "µs",
            "extra": "min 28111.2 µs, p95 32951.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 30357.6,
            "unit": "µs",
            "extra": "min 28279.4 µs, p95 35311 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 32137.5,
            "unit": "µs",
            "extra": "min 30625.9 µs, p95 34975.6 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 291.5,
            "unit": "µs",
            "extra": "min 280.5 µs, p95 303.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 326.2,
            "unit": "µs",
            "extra": "min 291.7 µs, p95 345.6 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 284.4,
            "unit": "µs",
            "extra": "min 269.4 µs, p95 303.8 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 13875.6,
            "unit": "µs",
            "extra": "min 12243.7 µs, p95 16661.6 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 12392.9,
            "unit": "µs",
            "extra": "min 11900 µs, p95 13865.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 14,
            "unit": "µs",
            "extra": "min 13 µs, p95 14.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 15.6,
            "unit": "µs",
            "extra": "min 15.3 µs, p95 15.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 20922.7,
            "unit": "µs",
            "extra": "min 18293.7 µs, p95 25765.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 21567.1,
            "unit": "µs",
            "extra": "min 19046.6 µs, p95 26340.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12797.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 13020.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 55.3,
            "unit": "µs",
            "extra": "min 53 µs, p95 71.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 68,
            "unit": "µs",
            "extra": "min 63.1 µs, p95 79.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 11.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 25.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 53.4,
            "unit": "µs",
            "extra": "min 50.3 µs, p95 64.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 72.1,
            "unit": "µs",
            "extra": "min 64.5 µs, p95 82.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 10.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 29.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 202.1,
            "unit": "µs",
            "extra": "min 167.2 µs, p95 340.2 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 298.7,
            "unit": "µs",
            "extra": "min 269.1 µs, p95 404.2 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 26890.4,
            "unit": "µs",
            "extra": "min 25544.4 µs, p95 28374.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 28655.7,
            "unit": "µs",
            "extra": "min 26671.8 µs, p95 31762.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 32219.1,
            "unit": "µs",
            "extra": "min 29725.1 µs, p95 33782.7 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 178.9,
            "unit": "µs",
            "extra": "min 170.5 µs, p95 362.1 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 225.1,
            "unit": "µs",
            "extra": "min 218.5 µs, p95 244.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 231.6,
            "unit": "µs",
            "extra": "min 226.5 µs, p95 243.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.35,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.56,
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
            "value": 0.1,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.117,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 7.14,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.524,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 26.85,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 25.14,
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
            "value": 0.2,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.29,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.29,
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
          "id": "37282376c5d1521d5a6a11b62220eceb12c8dca7",
          "message": "test: record today's second Content-Type rule check as S1.26\n\nFound landing S1.23. Today's verifier checks a rule on the pact's\nContent-Type header on the header as sent, and again before it compares a\nbody, against the reply's type as pact_models writes it: no space before a\nparameter, charset in lower case. A regex written with the space passes the\nfirst check and fails the second. janus verify checks the rule once, on the\nheader (ADR 0036 decision 4), and passes.\n\nThe fixture bodies/content-type-rule-with-a-space shows it on a JSON body,\nso it is not multipart's alone, and its two ledger entries name S1.26, a new\nplan item that decides whether the legacy compiler copies the second check\nor an ADR makes the difference deliberate. Nothing the engine ships changes.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T17:44:22+11:00",
          "tree_id": "789ac731a096dd03b98419ef52cbcf8dd49db9aa",
          "url": "https://github.com/rholshausen/pact-janus/commit/37282376c5d1521d5a6a11b62220eceb12c8dca7"
        },
        "date": 1792651761326,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8,
            "unit": "µs",
            "extra": "min 7.6 µs, p95 11.3 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1924.7,
            "unit": "µs",
            "extra": "min 1793.3 µs, p95 2053.3 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 159.9,
            "unit": "µs",
            "extra": "min 107.5 µs, p95 232.5 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2110.2,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8189668,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 42606.5,
            "unit": "µs",
            "extra": "min 38455 µs, p95 52219 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 43433,
            "unit": "µs",
            "extra": "min 38528 µs, p95 54380.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 44526.2,
            "unit": "µs",
            "extra": "min 42217.2 µs, p95 54143.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 399.1,
            "unit": "µs",
            "extra": "min 382.5 µs, p95 414.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 444.2,
            "unit": "µs",
            "extra": "min 405.2 µs, p95 507.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 425.1,
            "unit": "µs",
            "extra": "min 400.7 µs, p95 453.4 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 20948,
            "unit": "µs",
            "extra": "min 16560.8 µs, p95 28372.6 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 19258.8,
            "unit": "µs",
            "extra": "min 16600.3 µs, p95 22448.7 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 18,
            "unit": "µs",
            "extra": "min 17.2 µs, p95 18.5 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 22.3,
            "unit": "µs",
            "extra": "min 21.8 µs, p95 22.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 29764.8,
            "unit": "µs",
            "extra": "min 24725.5 µs, p95 37844.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 30243.5,
            "unit": "µs",
            "extra": "min 25116 µs, p95 41112.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 18420.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 18574.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 101.3,
            "unit": "µs",
            "extra": "min 93.4 µs, p95 120.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 128.8,
            "unit": "µs",
            "extra": "min 113.7 µs, p95 150.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 22.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 50.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 83.3,
            "unit": "µs",
            "extra": "min 74.1 µs, p95 111.6 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 115.1,
            "unit": "µs",
            "extra": "min 102.5 µs, p95 184 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 16.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 45.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 296.7,
            "unit": "µs",
            "extra": "min 269.6 µs, p95 339.4 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 430.3,
            "unit": "µs",
            "extra": "min 367.1 µs, p95 634.2 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 39927,
            "unit": "µs",
            "extra": "min 35423 µs, p95 50850.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 41561.6,
            "unit": "µs",
            "extra": "min 35780.9 µs, p95 48201.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 45455,
            "unit": "µs",
            "extra": "min 43005.7 µs, p95 54121.5 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 254.2,
            "unit": "µs",
            "extra": "min 222.6 µs, p95 302.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 295,
            "unit": "µs",
            "extra": "min 279.3 µs, p95 360.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 341.1,
            "unit": "µs",
            "extra": "min 328.9 µs, p95 362.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.82,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.4,
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
            "value": 0.147,
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
            "value": 10.1,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.657,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 38.09,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 35.25,
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
            "value": 0.35,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.44,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "4057e1433a8668942599a644f6ff9971c9633e9d",
          "message": "fix: generated bindings keep the schema's member order however the generator is built\n\nThe release builds janus-sdk-gates alongside the CLI, so Cargo unified the\nkernel's serde_json `preserve_order` (ADR 0037, S1.20) into the bindings\ngenerator: the release's gates wrote members in schema order while\n`cargo run -p pact_janus_bindings` sorted them, and the release's bindings\ngate failed on both SDKs. The bindings crate now asks for `preserve_order`\nitself, and the checked-in bindings are regenerated in schema order.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T18:20:05+11:00",
          "tree_id": "0ead9bb71c37035b34eee234b3e2c6e86df4fbeb",
          "url": "https://github.com/rholshausen/pact-janus/commit/4057e1433a8668942599a644f6ff9971c9633e9d"
        },
        "date": 1792654463707,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10.6,
            "unit": "µs",
            "extra": "min 10 µs, p95 14.9 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2390.7,
            "unit": "µs",
            "extra": "min 2277.8 µs, p95 2429 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 234.4,
            "unit": "µs",
            "extra": "min 213.9 µs, p95 261.2 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2547.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8189668,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 47013.2,
            "unit": "µs",
            "extra": "min 45431.6 µs, p95 48907.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 47826.8,
            "unit": "µs",
            "extra": "min 46683.4 µs, p95 49257.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 53285.9,
            "unit": "µs",
            "extra": "min 52749.7 µs, p95 54057.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 514.9,
            "unit": "µs",
            "extra": "min 497.2 µs, p95 534.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 536.6,
            "unit": "µs",
            "extra": "min 514.1 µs, p95 598.2 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 546.6,
            "unit": "µs",
            "extra": "min 512.7 µs, p95 585.7 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 20724.4,
            "unit": "µs",
            "extra": "min 19657.6 µs, p95 22743.5 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 21278.9,
            "unit": "µs",
            "extra": "min 20956.7 µs, p95 21914.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 23.4,
            "unit": "µs",
            "extra": "min 22.3 µs, p95 24.1 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 28.4,
            "unit": "µs",
            "extra": "min 27.7 µs, p95 29.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 29908.5,
            "unit": "µs",
            "extra": "min 27903 µs, p95 31560 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 31087.8,
            "unit": "µs",
            "extra": "min 29975.8 µs, p95 37829.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 17356.5,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 18320.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 112.8,
            "unit": "µs",
            "extra": "min 100.1 µs, p95 134.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 143.3,
            "unit": "µs",
            "extra": "min 129.1 µs, p95 162.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 23.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 53,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 101.2,
            "unit": "µs",
            "extra": "min 94.4 µs, p95 133.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 139.1,
            "unit": "µs",
            "extra": "min 121.5 µs, p95 157.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 20.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 55.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 382.2,
            "unit": "µs",
            "extra": "min 315.9 µs, p95 515.5 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 469.6,
            "unit": "µs",
            "extra": "min 435.2 µs, p95 589.8 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 44150.8,
            "unit": "µs",
            "extra": "min 42754 µs, p95 46325.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 45227,
            "unit": "µs",
            "extra": "min 43494.1 µs, p95 47556.2 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 53294.5,
            "unit": "µs",
            "extra": "min 52708.1 µs, p95 54273.4 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 300.4,
            "unit": "µs",
            "extra": "min 286.2 µs, p95 316.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 374.3,
            "unit": "µs",
            "extra": "min 356.2 µs, p95 403.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 441.1,
            "unit": "µs",
            "extra": "min 420.4 µs, p95 466.5 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.26,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.57,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.38,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.42,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.186,
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
            "value": 12.1,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.855,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 37.54,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 37.84,
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
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "1bb2c01bf3c316eba4bd5a3f771000530a96d1f1",
          "message": "chore: release 0.3.0",
          "timestamp": "2026-10-22T18:49:44+11:00",
          "tree_id": "20eda7a8696566c70e416d916648bd143ae8d221",
          "url": "https://github.com/rholshausen/pact-janus/commit/1bb2c01bf3c316eba4bd5a3f771000530a96d1f1"
        },
        "date": 1792656236273,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10.4,
            "unit": "µs",
            "extra": "min 9.8 µs, p95 39.7 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2170.6,
            "unit": "µs",
            "extra": "min 2094.5 µs, p95 2496 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 222.3,
            "unit": "µs",
            "extra": "min 186.6 µs, p95 316.4 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2621.7,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8189545,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 41774.3,
            "unit": "µs",
            "extra": "min 40812.3 µs, p95 44133.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 43401.5,
            "unit": "µs",
            "extra": "min 42574 µs, p95 44431.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 50919.9,
            "unit": "µs",
            "extra": "min 50498.8 µs, p95 51667.2 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 505.9,
            "unit": "µs",
            "extra": "min 479.3 µs, p95 531.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 570.7,
            "unit": "µs",
            "extra": "min 526.8 µs, p95 606.5 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 579.3,
            "unit": "µs",
            "extra": "min 553.4 µs, p95 599.2 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 19464.2,
            "unit": "µs",
            "extra": "min 18043.6 µs, p95 20610.4 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 21700.5,
            "unit": "µs",
            "extra": "min 21421.6 µs, p95 22391.3 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 23.6,
            "unit": "µs",
            "extra": "min 21.8 µs, p95 24.9 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 28.3,
            "unit": "µs",
            "extra": "min 27.9 µs, p95 29.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 28349,
            "unit": "µs",
            "extra": "min 27048.5 µs, p95 30188.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 27840.1,
            "unit": "µs",
            "extra": "min 26579.5 µs, p95 30368.3 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 16718.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 16068.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 138.5,
            "unit": "µs",
            "extra": "min 134 µs, p95 150.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 174,
            "unit": "µs",
            "extra": "min 162.2 µs, p95 190.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 24.7,
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
            "value": 140,
            "unit": "µs",
            "extra": "min 124.2 µs, p95 164 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 180.3,
            "unit": "µs",
            "extra": "min 154.6 µs, p95 197 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 27.3,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 70.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 445.4,
            "unit": "µs",
            "extra": "min 339.6 µs, p95 588.3 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 520.1,
            "unit": "µs",
            "extra": "min 415.3 µs, p95 656 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 38693.4,
            "unit": "µs",
            "extra": "min 38087.7 µs, p95 40314.7 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 40610.1,
            "unit": "µs",
            "extra": "min 39993.3 µs, p95 41637.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 50676.5,
            "unit": "µs",
            "extra": "min 50213.5 µs, p95 51443.1 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 322.1,
            "unit": "µs",
            "extra": "min 307.4 µs, p95 345.7 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 413.3,
            "unit": "µs",
            "extra": "min 389.2 µs, p95 432.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 453.2,
            "unit": "µs",
            "extra": "min 437.3 µs, p95 472.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.32,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.74,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.37,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.41,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.194,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.223,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 11.44,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.907,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 33.13,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 32.91,
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
            "value": 0.5,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "7de3724e7af2d45f501bfd9884afc48338adeb67",
          "message": "feat: S1.17 a filtered run is reported as filtered, never verified, and is not published\n\nA --variant naming an id no document recorded replayed nothing and reported\nVERIFIED with exit 0; from a broker with --publish it published a success\nfor every pact, since a pact with no results had none that failed. ADR 0041\ndecides what a filtered run is and how it reads:\n\n- A filter naming an id no document records is variant-not-found, with the\n  first such id and the ids the documents record. verify refuses it for an\n  inline source (janus verify exits 2); a broker run ends right after\n  verification/fetched, before anything runs or is published. An empty\n  variants list is malformed-frame.\n- A run is filtered when its filter left out a recorded variant, as pact-jvm\n  judges it; one that admits everything (base, over v1-v4 pacts) is whole.\n- A filtered run that nothing failed has status filtered, and so does each\n  pairs entry it narrowed. janus verify prints FILTERED and exits 0: it did\n  what it was asked, as today's verifiers do.\n- A narrowed pact is not published: outcome skipped, reason filtered, as\n  pact-jvm skips it. pact_verifier_cli 1.3.5 publishes it, and passes a\n  filter that matches nothing with exit 0.\n- subsumption/decide raises verification-filtered per pair.\n\nThe variant-semantics and engine-protocol specs say so. The cases are\nprotocol-level tests in the kernel's verification and broker tests, and two\nCLI tests replace the snapshot that recorded the gap; corpora/ holds plans\nand conformance/ has no provider side until S1.14, which gains them. S1.13's\n--filter-* rows carry ADR 0041.\n\nAlso: S1.18's Done line in the staged plan (178cb64), and start()'s doc\ncomment, which had drifted onto RunComponents.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T10:24:25+11:00",
          "tree_id": "4cf962543227f55092ed4346084d123eae52adfc",
          "url": "https://github.com/rholshausen/pact-janus/commit/7de3724e7af2d45f501bfd9884afc48338adeb67"
        },
        "date": 1792712401963,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.9,
            "unit": "µs",
            "extra": "min 7.7 µs, p95 14.2 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1891.7,
            "unit": "µs",
            "extra": "min 1795.6 µs, p95 1980.4 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 132.3,
            "unit": "µs",
            "extra": "min 112.3 µs, p95 191.4 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2005.3,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8208623,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 40494.7,
            "unit": "µs",
            "extra": "min 37535.1 µs, p95 42608.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 40706.1,
            "unit": "µs",
            "extra": "min 39071.8 µs, p95 42756.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 41719.1,
            "unit": "µs",
            "extra": "min 41135.2 µs, p95 42462.7 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 402.9,
            "unit": "µs",
            "extra": "min 388 µs, p95 418.4 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 455,
            "unit": "µs",
            "extra": "min 401.4 µs, p95 473.9 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 421.2,
            "unit": "µs",
            "extra": "min 398.5 µs, p95 447.3 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 19684.6,
            "unit": "µs",
            "extra": "min 16593.6 µs, p95 22683.4 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 16369.1,
            "unit": "µs",
            "extra": "min 16036.3 µs, p95 16717.1 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 19.1,
            "unit": "µs",
            "extra": "min 17.2 µs, p95 19.5 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 22.2,
            "unit": "µs",
            "extra": "min 21.7 µs, p95 22.7 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 26503,
            "unit": "µs",
            "extra": "min 24969.1 µs, p95 28337.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 27795,
            "unit": "µs",
            "extra": "min 23392.1 µs, p95 29589.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 15402,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 16848.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 82.3,
            "unit": "µs",
            "extra": "min 79.7 µs, p95 93.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 110.9,
            "unit": "µs",
            "extra": "min 99.3 µs, p95 130.1 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 15.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 40,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 88.8,
            "unit": "µs",
            "extra": "min 72.7 µs, p95 104.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 107.6,
            "unit": "µs",
            "extra": "min 96.8 µs, p95 128.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 20.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 42,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 294.4,
            "unit": "µs",
            "extra": "min 230.3 µs, p95 420.6 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 390.8,
            "unit": "µs",
            "extra": "min 328.7 µs, p95 544.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 37500.2,
            "unit": "µs",
            "extra": "min 35496.5 µs, p95 39352.1 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 37819.3,
            "unit": "µs",
            "extra": "min 36206.3 µs, p95 40017 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 41588.2,
            "unit": "µs",
            "extra": "min 40673 µs, p95 42360.3 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 239.3,
            "unit": "µs",
            "extra": "min 232.1 µs, p95 252.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 288.1,
            "unit": "µs",
            "extra": "min 276 µs, p95 344.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 337.6,
            "unit": "µs",
            "extra": "min 326.4 µs, p95 354.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.75,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.01,
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
            "value": 0.32,
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
            "value": 0.156,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 9.27,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.644,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 28.27,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 29.4,
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
            "value": 0.32,
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
            "value": 0.44,
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
          "id": "2f4150297cc97156526009a32fed5c6e26ce0739",
          "message": "feat: S1.24 values, eachKey, eachValue and arrayContains verify as today\n\nA v1-v4 pact whose body used one of these four rules failed every\ninteraction that had one: the legacy compiler compiled each to flow:error.\nNo pact-specification case covers them. ADR 0042 compiles them as\npact_matching's compare_maps_with_matchingrule and\ncompare_lists_with_matchingrule apply them, on a body's objects and arrays:\n\n- values, eachValue and arrayContains on an object are a flow:for-each over\n  its entries, dispatching on ~>.key: each entry against the pact's value\n  for its key, or the pact's first value by key; keys are not compared.\n  eachValue swaps in its own rules at <path>.*. Branches are labelled as the\n  member, or {*} for any other key.\n- eachKey holds every key to its rules (labelled {key}), and compares the\n  pact's own members only where the reply has them.\n- arrayContains on an array is one legacy:array-contains per variant. The\n  action was in the grammar and never emitted; it now runs a template, lets\n  one element satisfy two variants (match:contains does not), and fails\n  with today's \"Variant at index 0 (...) was not found in the actual list\".\n  Its template is labelled [?i], specified in plan-grammar spec 4.4.\n- eachValue on an array holds every element to its rules, and checks\n  nothing when cascaded. On a scalar, values is equality and the other\n  three accept any value, as today's JSON matcher does.\n\nS1.1's new matching-rules/ fixtures, one per rule with replies that match\nand that do not and mutations, give 379 verdicts over 183 cases, all\nagreeing with pact_verifier_cli 1.3.5; the whole harness reads 780\nverdicts, 768 agreeing and 12 known. Corpus cases: legacy/v3-values,\nv4-each-key, v4-each-value, v4-array-contains and their -mismatch twins.\n\nLeft out: the four declared on text (path, header, query, form, XML) are\nstill refused; janus upgrade still converts them approximately (S1.25);\nmismatches inside entry templates report relative paths (~>.value.id),\nnoted for the CLI output review.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T11:19:52+11:00",
          "tree_id": "3eea76c7387557ca880fb5cf83ae049d5c3df3fa",
          "url": "https://github.com/rholshausen/pact-janus/commit/2f4150297cc97156526009a32fed5c6e26ce0739"
        },
        "date": 1792715058002,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 8.2,
            "unit": "µs",
            "extra": "min 5.4 µs, p95 10.7 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1787.3,
            "unit": "µs",
            "extra": "min 1676.1 µs, p95 1945.9 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 139.6,
            "unit": "µs",
            "extra": "min 121.6 µs, p95 154 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1620,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8289851,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 36306.8,
            "unit": "µs",
            "extra": "min 33309.6 µs, p95 40007.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 35824.7,
            "unit": "µs",
            "extra": "min 31855.9 µs, p95 38128.8 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 35431.6,
            "unit": "µs",
            "extra": "min 32598.2 µs, p95 38982.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 300.6,
            "unit": "µs",
            "extra": "min 294.2 µs, p95 310.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 311.2,
            "unit": "µs",
            "extra": "min 302 µs, p95 325.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 299.7,
            "unit": "µs",
            "extra": "min 289.3 µs, p95 310.6 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 20743.5,
            "unit": "µs",
            "extra": "min 17316 µs, p95 23596.9 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 15072.6,
            "unit": "µs",
            "extra": "min 13177.1 µs, p95 16428.7 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 14,
            "unit": "µs",
            "extra": "min 13.2 µs, p95 14.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 16.1,
            "unit": "µs",
            "extra": "min 15.7 µs, p95 16.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 28671.2,
            "unit": "µs",
            "extra": "min 24693.5 µs, p95 32292.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 28240.5,
            "unit": "µs",
            "extra": "min 23946 µs, p95 34338.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 17896.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 17510,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 63,
            "unit": "µs",
            "extra": "min 54.9 µs, p95 78.8 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 79.7,
            "unit": "µs",
            "extra": "min 70.7 µs, p95 89.5 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 12.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 30.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 60.1,
            "unit": "µs",
            "extra": "min 56.1 µs, p95 67.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 79,
            "unit": "µs",
            "extra": "min 70.1 µs, p95 87.2 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 10.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 30.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 233.9,
            "unit": "µs",
            "extra": "min 180.5 µs, p95 346.2 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 287.4,
            "unit": "µs",
            "extra": "min 257.2 µs, p95 434.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 33356.2,
            "unit": "µs",
            "extra": "min 29503.1 µs, p95 39574.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 33750,
            "unit": "µs",
            "extra": "min 30514.8 µs, p95 37211.4 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 35318.8,
            "unit": "µs",
            "extra": "min 32709.6 µs, p95 38238.4 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 181.2,
            "unit": "µs",
            "extra": "min 175.2 µs, p95 191.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 239.9,
            "unit": "µs",
            "extra": "min 218.1 µs, p95 255.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 247.6,
            "unit": "µs",
            "extra": "min 241.1 µs, p95 259 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.42,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.58,
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
            "value": 0.27,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.109,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.118,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 8.28,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.542,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 31.33,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 33.13,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.22,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.23,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.33,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.34,
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
          "id": "ea1f4368e80ec5f43e3cdbd6fb5ed2725324ddfb",
          "message": "docs: S1.26 a pact's Content-Type is checked once, on the header (ADR 0043)\n\nToday's verifier checks a named Content-Type a second time before the body:\nwhen the pact's type has parameters, the reply must have exactly those, or\nthe header's rule must match both types as pact_models writes them (no\nspace, charset in lower case). It fails replies its own header check has\njust accepted: a rule written with a space and another charset, and,\nfound here, a reply that adds a parameter with no rule at all. pact-jvm\ncompares base types only and passes both, as janus verify does.\n\nADR 0043 makes the difference deliberate. bodies/content-type-extra-\nparameter is the new fixture, and the four ledger entries for it and\nbodies/content-type-rule-with-a-space carry the ADR. A probe of the\nneighbouring cases (charset case, a pact type with no parameters, a rule\nwith the same charset or written to allow no space) found them agreeing.\nThe harness reads 782 verdicts: 768 agree, 14 known. Nothing the engine\nships changes.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T11:42:39+11:00",
          "tree_id": "1b036ce771d696d21e8905552a1c3f417df416f7",
          "url": "https://github.com/rholshausen/pact-janus/commit/ea1f4368e80ec5f43e3cdbd6fb5ed2725324ddfb"
        },
        "date": 1792717291952,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 5.7,
            "unit": "µs",
            "extra": "min 5.3 µs, p95 8.2 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 1819.1,
            "unit": "µs",
            "extra": "min 1710.2 µs, p95 2055.3 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 108.3,
            "unit": "µs",
            "extra": "min 98.3 µs, p95 129.5 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 1605.8,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8289804,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 29461.5,
            "unit": "µs",
            "extra": "min 27638.3 µs, p95 31545.5 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 32539.3,
            "unit": "µs",
            "extra": "min 29098.1 µs, p95 35845 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 32300,
            "unit": "µs",
            "extra": "min 30208.5 µs, p95 34775.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 294.5,
            "unit": "µs",
            "extra": "min 286.7 µs, p95 315.7 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 325.8,
            "unit": "µs",
            "extra": "min 294 µs, p95 344.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 286.4,
            "unit": "µs",
            "extra": "min 272.4 µs, p95 466.9 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 15622.3,
            "unit": "µs",
            "extra": "min 11843 µs, p95 20029 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 13319.9,
            "unit": "µs",
            "extra": "min 12181 µs, p95 16093.3 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 13.6,
            "unit": "µs",
            "extra": "min 12.9 µs, p95 14 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 16.5,
            "unit": "µs",
            "extra": "min 15.4 µs, p95 74.9 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 20872.8,
            "unit": "µs",
            "extra": "min 18996.5 µs, p95 25383.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 25022.2,
            "unit": "µs",
            "extra": "min 19780.9 µs, p95 29042.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 12462.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 14809.4,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 62.7,
            "unit": "µs",
            "extra": "min 58.7 µs, p95 70 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 79.9,
            "unit": "µs",
            "extra": "min 69.7 µs, p95 86.4 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 12,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 30.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 61.1,
            "unit": "µs",
            "extra": "min 55 µs, p95 68.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 83.2,
            "unit": "µs",
            "extra": "min 69.1 µs, p95 94 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 10.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 33.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 238.7,
            "unit": "µs",
            "extra": "min 161.9 µs, p95 328.1 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 307.4,
            "unit": "µs",
            "extra": "min 268.7 µs, p95 492.4 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 26639.3,
            "unit": "µs",
            "extra": "min 25053.5 µs, p95 28635.5 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 29270.5,
            "unit": "µs",
            "extra": "min 26402.3 µs, p95 34259.6 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 32716.2,
            "unit": "µs",
            "extra": "min 30152.7 µs, p95 36456 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 178.9,
            "unit": "µs",
            "extra": "min 174.1 µs, p95 188.9 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 222,
            "unit": "µs",
            "extra": "min 213.8 µs, p95 237 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 323.2,
            "unit": "µs",
            "extra": "min 243.4 µs, p95 469.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 1.32,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 1.57,
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
            "value": 0.099,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.119,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 7.17,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.486,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 23.11,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 26.34,
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
            "value": 0.21,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (native)",
            "value": 0.29,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small-upgraded (subprocess)",
            "value": 0.32,
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
          "id": "31ece188f8ee180d738a072596d9b2ac3d0851f3",
          "message": "fix: the benchmark lock file follows hooks-host, and exec's tests are Unix-only on every target\n\nS1.9 changed hooks-host's dependencies without updating\nbenchmarks/janus/Cargo.lock, so --locked failed in the Benchmarks run and\nthe checks job. The exec deadline tests gated each item to Unix but not\ntheir imports, which Windows clippy refused as unused; the file is now\ngated as a whole.\n\nNo-Changeset: a lock file outside the release and a test file\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T13:23:29+11:00",
          "tree_id": "5b7c8829941a76f548a1851cc7a45b49c403b002",
          "url": "https://github.com/rholshausen/pact-janus/commit/31ece188f8ee180d738a072596d9b2ac3d0851f3"
        },
        "date": 1792722513119,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10.3,
            "unit": "µs",
            "extra": "min 9.9 µs, p95 16 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2173.3,
            "unit": "µs",
            "extra": "min 2094.7 µs, p95 2273.5 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 210.1,
            "unit": "µs",
            "extra": "min 169.5 µs, p95 490 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2683,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8268813,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 43685.2,
            "unit": "µs",
            "extra": "min 41879.6 µs, p95 45689.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 43748.1,
            "unit": "µs",
            "extra": "min 42634.6 µs, p95 46314.1 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 51792.3,
            "unit": "µs",
            "extra": "min 51378.2 µs, p95 52584.3 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 502,
            "unit": "µs",
            "extra": "min 480.4 µs, p95 519 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 581.6,
            "unit": "µs",
            "extra": "min 527.1 µs, p95 616.3 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 579.6,
            "unit": "µs",
            "extra": "min 554 µs, p95 602.1 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18688.4,
            "unit": "µs",
            "extra": "min 18088.2 µs, p95 19608.2 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 21706.6,
            "unit": "µs",
            "extra": "min 21197.7 µs, p95 22409.6 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 22.5,
            "unit": "µs",
            "extra": "min 21.7 µs, p95 23.2 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 29,
            "unit": "µs",
            "extra": "min 28.6 µs, p95 30 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 29539.5,
            "unit": "µs",
            "extra": "min 26818.3 µs, p95 32593.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 28347.3,
            "unit": "µs",
            "extra": "min 26608.1 µs, p95 35037 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 17123.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 16307.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 145.5,
            "unit": "µs",
            "extra": "min 141.5 µs, p95 151.6 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 190.3,
            "unit": "µs",
            "extra": "min 166.3 µs, p95 198.7 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 26.2,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 68.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 139.7,
            "unit": "µs",
            "extra": "min 127.7 µs, p95 151.4 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 181.5,
            "unit": "µs",
            "extra": "min 157.4 µs, p95 194.5 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 25.6,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 67.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 431.1,
            "unit": "µs",
            "extra": "min 313.9 µs, p95 585.2 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 495.9,
            "unit": "µs",
            "extra": "min 401.6 µs, p95 644.8 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 39882,
            "unit": "µs",
            "extra": "min 39477.2 µs, p95 40499.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 41761.2,
            "unit": "µs",
            "extra": "min 40548.2 µs, p95 43655.8 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 51669.5,
            "unit": "µs",
            "extra": "min 50960.3 µs, p95 52445.5 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 311.3,
            "unit": "µs",
            "extra": "min 299.4 µs, p95 335.4 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 449,
            "unit": "µs",
            "extra": "min 384.5 µs, p95 485.2 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 452.1,
            "unit": "µs",
            "extra": "min 429.4 µs, p95 494.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.36,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.93,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.36,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.42,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.198,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (subprocess)",
            "value": 0.247,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-provider-order-service (native)",
            "value": 9.49,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.561,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 32.36,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 34.88,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-small (native)",
            "value": 0.36,
            "unit": "ms per interaction",
            "extra": "runs 5, interactions 50"
          },
          {
            "name": "verify-corpus-small (subprocess)",
            "value": 0.42,
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
            "email": "ronald.holshausen@smartbear.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "3094b45487990cf62fb3c45e0f3fa3f0a6c5a2c1",
          "message": "feat: S1.8 an interaction that needs a pact plugin is refused by name, and the rest of its pact verifies (ADR 0045)\n\nA v1-v4 interaction needs a pact plugin when its pluginConfiguration names\none, or when it uses a rule pact_models reads as a plugin's and the pact\nnames plugins (then it needs all of them, as today's verifier loads).\n\n- janus verify refuses each such interaction before any state or request,\n  as component-unavailable naming pact-plugin/<name>; the rest of the pact\n  verifies, and the summary counts the refusal as a failure.\n- janus upgrade drops it with an interaction-dropped finding naming the\n  plugin; janus explain refuses it with the same error.\n- Previously a CSV/JWT/credit-card HTTP interaction was verified only in\n  part with the built-in components, and a gRPC/Avro message one was\n  skipped silently.\n\nNo S1.1 fixture: today's verifier installs a missing plugin from the\ninternet. The refusals are tested offline in the kernel and the CLI.\nFixtures are trimmed from pact-plugins' examples (MIT).\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T16:55:17+11:00",
          "tree_id": "17b04a6ec47e8f14fb2dccd778bdf314a3bc96cf",
          "url": "https://github.com/rholshausen/pact-janus/commit/3094b45487990cf62fb3c45e0f3fa3f0a6c5a2c1"
        },
        "date": 1792735223693,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "cold-start (native)",
            "value": 10.1,
            "unit": "µs",
            "extra": "min 9.9 µs, p95 14.5 µs, iterations 20"
          },
          {
            "name": "cold-start (subprocess)",
            "value": 2153.8,
            "unit": "µs",
            "extra": "min 2066.8 µs, p95 2247.7 µs, iterations 20"
          },
          {
            "name": "cold-start (wasm)",
            "value": 222.7,
            "unit": "µs",
            "extra": "min 175.9 µs, p95 319.4 µs, iterations 20"
          },
          {
            "name": "cold-start · compile (wasm)",
            "value": 2647,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "cold-start · component size (wasm)",
            "value": 8310492,
            "unit": "bytes",
            "extra": ""
          },
          {
            "name": "explain-pact-100k (native)",
            "value": 42498.8,
            "unit": "µs",
            "extra": "min 41446.9 µs, p95 43875 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (subprocess)",
            "value": 43274.1,
            "unit": "µs",
            "extra": "min 41840.1 µs, p95 44440.9 µs, iterations 200"
          },
          {
            "name": "explain-pact-100k (wasm)",
            "value": 51952.6,
            "unit": "µs",
            "extra": "min 51025.2 µs, p95 52810.4 µs, iterations 200"
          },
          {
            "name": "explain-pact-small (native)",
            "value": 507.8,
            "unit": "µs",
            "extra": "min 486.5 µs, p95 537.1 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (subprocess)",
            "value": 582,
            "unit": "µs",
            "extra": "min 529.5 µs, p95 609.8 µs, iterations 2000"
          },
          {
            "name": "explain-pact-small (wasm)",
            "value": 593.6,
            "unit": "µs",
            "extra": "min 567.6 µs, p95 613 µs, iterations 2000"
          },
          {
            "name": "match-100k (native)",
            "value": 18799.4,
            "unit": "µs",
            "extra": "min 18153.6 µs, p95 21281.1 µs, iterations 500"
          },
          {
            "name": "match-100k (wasm)",
            "value": 21637.6,
            "unit": "µs",
            "extra": "min 20945.1 µs, p95 22778.7 µs, iterations 500"
          },
          {
            "name": "match-small (native)",
            "value": 22.5,
            "unit": "µs",
            "extra": "min 20.9 µs, p95 23.3 µs, iterations 5000"
          },
          {
            "name": "match-small (wasm)",
            "value": 28.8,
            "unit": "µs",
            "extra": "min 28.4 µs, p95 29.8 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-100k (native)",
            "value": 27723.6,
            "unit": "µs",
            "extra": "min 26131.5 µs, p95 30375.2 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k (subprocess)",
            "value": 28596.8,
            "unit": "µs",
            "extra": "min 26140.2 µs, p95 34673 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-100k · arming (native)",
            "value": 15984.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-100k · arming (subprocess)",
            "value": 16456.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch (native)",
            "value": 142.2,
            "unit": "µs",
            "extra": "min 130.7 µs, p95 152.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch (subprocess)",
            "value": 181.4,
            "unit": "µs",
            "extra": "min 157.9 µs, p95 192.9 µs, iterations 500"
          },
          {
            "name": "mock-server-latency-mismatch · arming (native)",
            "value": 25.7,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-mismatch · arming (subprocess)",
            "value": 66.1,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small (native)",
            "value": 138.1,
            "unit": "µs",
            "extra": "min 122 µs, p95 150.1 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small (subprocess)",
            "value": 179.4,
            "unit": "µs",
            "extra": "min 144.8 µs, p95 204.3 µs, iterations 5000"
          },
          {
            "name": "mock-server-latency-small · arming (native)",
            "value": 24.9,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-latency-small · arming (subprocess)",
            "value": 68.8,
            "unit": "µs",
            "extra": ""
          },
          {
            "name": "mock-server-startup-cycle (native)",
            "value": 402.2,
            "unit": "µs",
            "extra": "min 331.3 µs, p95 577.8 µs, iterations 20"
          },
          {
            "name": "mock-server-startup-cycle (subprocess)",
            "value": 524.3,
            "unit": "µs",
            "extra": "min 436.5 µs, p95 695.1 µs, iterations 20"
          },
          {
            "name": "upgrade-pact-100k (native)",
            "value": 39246.6,
            "unit": "µs",
            "extra": "min 38485.6 µs, p95 40307.9 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (subprocess)",
            "value": 40765.4,
            "unit": "µs",
            "extra": "min 39707.1 µs, p95 42393.3 µs, iterations 200"
          },
          {
            "name": "upgrade-pact-100k (wasm)",
            "value": 51539.4,
            "unit": "µs",
            "extra": "min 51008.2 µs, p95 52578.1 µs, iterations 200"
          },
          {
            "name": "variant-space-rfc-order (native)",
            "value": 311.8,
            "unit": "µs",
            "extra": "min 301.3 µs, p95 335.6 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (subprocess)",
            "value": 456,
            "unit": "µs",
            "extra": "min 383.8 µs, p95 480.8 µs, iterations 200, variants 8"
          },
          {
            "name": "variant-space-rfc-order (wasm)",
            "value": 453.4,
            "unit": "µs",
            "extra": "min 435.1 µs, p95 474.3 µs, iterations 200, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (native)",
            "value": 2.35,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order (subprocess)",
            "value": 2.74,
            "unit": "ms",
            "extra": "runs 20, variants 8"
          },
          {
            "name": "variants-consumer-rfc-order · finalise (native)",
            "value": 0.37,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · finalise (subprocess)",
            "value": 0.41,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "variants-consumer-rfc-order · per extra variant (native)",
            "value": 0.196,
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
            "value": 9.46,
            "unit": "ms",
            "extra": "runs 10, variants 6"
          },
          {
            "name": "variants-provider-order-service · per extra variant (native)",
            "value": 0.565,
            "unit": "ms",
            "extra": ""
          },
          {
            "name": "verify-corpus-100k (native)",
            "value": 32.86,
            "unit": "ms per interaction",
            "extra": "runs 3, interactions 5"
          },
          {
            "name": "verify-corpus-100k (subprocess)",
            "value": 32.68,
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
            "value": 0.41,
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
            "value": 0.55,
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
          "id": "3c0bd6b06f548b4672c83173843ecaab938d049a",
          "message": "feat: S1.3 a run's summary counts its variants per source document\n\nPhase 9 finding 10, option A, decided in ADR 0033 and built.\n\n- The verification summary gains `pairs`, positionally parallel to\n  `consumers`/`providers`: per source document its consumer, provider,\n  status (verified, failed, incomplete) and its own variant counts,\n  which the run-level `variants` adds up. A pending document that\n  failed is `failed` there; the run's `status` is what forgives it.\n- Per source document rather than per (consumer, provider): a broker\n  run's main and deployed pacts for one consumer are different matrix\n  rows, and a sum would hide which one failed.\n- subsumption/decide reads the entries: a pair's counts are its\n  entries' added up, present whenever one run covered it, so janus\n  check reports counts for multi-contract runs. A summary from before\n  `pairs` is read as it was.\n- ADR 0033 indexed and dropped from the backlog; spec §9.6, the broker\n  worked example and subsumption.schema.json's descriptions updated,\n  and the bindings regenerated (doc comments only).\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T12:07:11+11:00",
          "tree_id": "0ba2d99b7e2a8cdf9b36e9b67f0c58bb3458c3a0",
          "url": "https://github.com/rholshausen/pact-janus/commit/3c0bd6b06f548b4672c83173843ecaab938d049a"
        },
        "date": 1792372305614,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 32912,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 30274,
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
          "id": "5d7f8a00cd5d63752deaadc73ec413cffb0e72d2",
          "message": "feat: S1.3 janus publish publishes a consumer's contracts to a Pact Broker\n\nbroker/publish-contracts is built (engine-protocol spec §8.7, ADR 0032): the kernel's broker\ntrait gains publish_contracts, the native client posts every contract to the index's\npb:publish-contracts link as the broker's 'pact' specification, and the engine reports the\noutcome in a broker session: one broker/published per contract, then broker/finished with the\nbroker's notices. Documents are read as verify reads them before anything is published, and are\npublished as sent.\n\njanus publish drives it with pact-broker publish's spellings and environment variables.\n\nCompression: the pinned broker (2.121.2) refuses a gzipped body with a 400 and advertises no\nAccept-Encoding, so the client gzips only when a broker says it can (RFC 7694) and resends\nuncompressed after a 415. Recorded in S1.3, the protocol spec and the contract-file spec.\n\nA live test publishes a pact and its upgraded Janus contract to the pinned broker and verifies\nboth back from it; CI's verifier-diff job runs it with JANUS_PACT_BROKER.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T14:21:37+11:00",
          "tree_id": "d04eff1e5ca901dbca9b87aff04feb969ed7b7a9",
          "url": "https://github.com/rholshausen/pact-janus/commit/5d7f8a00cd5d63752deaadc73ec413cffb0e72d2"
        },
        "date": 1792380354357,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28831,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21879,
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
          "id": "d206f4557fe71d1eaa3005d3c0b2a60e7a2f30c0",
          "message": "fix: S1.3 the broker client retries a 429 only when the broker says when\n\nPactFlow answers 429 when a publication would create an integration past the plan's limit, and\nno retry gets past that. The client now retries a 429 only when it carries Retry-After, after that\ndelay; a 5xx keeps the client's own backoff unless the broker asks for a delay of its own; and a\nRetry-After longer than 10 seconds is reported at once, with the delay, rather than waited for.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T14:42:11+11:00",
          "tree_id": "3cc7f258c0fd9a2fa803dfd33485166be3ee95e6",
          "url": "https://github.com/rholshausen/pact-janus/commit/d206f4557fe71d1eaa3005d3c0b2a60e7a2f30c0"
        },
        "date": 1792382202521,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 27626,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21903,
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
          "id": "2afc74f288b61ccda9d1ad2d9d7a91067b6a79b6",
          "message": "test: S1.3 the broker client replays a scenario recorded from the pinned broker\n\nThe second test layer: one scenario (a v3 pact and a Janus contract published, fetched with\npending and WIP, results published, fetched again, a changed republish refused) recorded from\nPact Broker 2.121.2 into engine/broker-client/tests/recorded/, and replayed offline on every\nsystem: each request must be the recorded one, and each step's conclusion the recorded one. The\nbroker's URL, timestamps and database ids are normalised, so a recording is the same file however\noften it is made; CI's verifier-diff job records it again and fails on any difference.\n\nThe hand-written cases from pact_verifier's own list: a body that is not JSON, a broker too old\nfor pacts-for-verification, a selection that is not HAL or selects nothing, selectors refused with\na 400, and a broker at a context path.\n\nThey found a difference, fixed here: today's verifier fetches every link from the broker it was\ngiven, keeping the link's path and query and adding a missing context path, so a broker behind a\nproxy that writes an internal host still works. The client now does the same, with pact_verifier's\nown cases (normalize_path_inner, resolve_path_test).\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T15:26:49+11:00",
          "tree_id": "f2e2ee5f9b0d7926c29e603c69a17b25406d7f0d",
          "url": "https://github.com/rholshausen/pact-janus/commit/2afc74f288b61ccda9d1ad2d9d7a91067b6a79b6"
        },
        "date": 1792384248648,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 40576,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 30142,
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
          "id": "2e219d8173b7410b070925f94f74d56ff93e4c0c",
          "message": "test: S1.3 the broker client's own pact with the Pact Broker, verified by the broker\n\nThe third test layer: engine/broker-client/tests/pact.rs writes the client's consumer pact with\nthe provider \"Pact Broker\" (pact_consumer) and checks the client against it on every system; in\nCI's verifier-diff job today's verifier verifies it against the pinned broker, each provider state\nset up through the broker's API by an endpoint of the test's own. Eleven interactions: the index;\npacts for verification with a pending pact and with a work-in-progress one, and fetching each; the\nprovider, its branch and tag; a verification result; publishing contracts, and a changed republish\nrefused. The parties and state names are the Ruby broker's own (Condor, the Pricing Service), so the\npact can be proposed upstream as it stands. Requests are exact; responses match by type, on only\nwhat the client reads.\n\npact_consumer is pinned to 1.4.10, the release that reads the pact_models the kernel uses, with no\ndefault features (no plugin driver). pact_models moves from 1.3.14 to 1.3.15, whose one change is\nthe fix for interactions duplicated on merge (pact-reference #550), which the pact's scenarios hit;\nthe kernel reads pacts and never merges them, and the corpora and the differential harness are\nunchanged. .cargo/config.toml sets PACT_DO_NOT_TRACK, so no test run sends pact_consumer's usage\nmetrics.\n\nNo-Changeset: test-only, and the one shipped dependency that moves changes only pact merging, which\nthe engine never does.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T15:54:05+11:00",
          "tree_id": "d40971a3d72ecdfa9b2ad6937ee2efbd4ad5b94e",
          "url": "https://github.com/rholshausen/pact-janus/commit/2e219d8173b7410b070925f94f74d56ff93e4c0c"
        },
        "date": 1792385926605,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 48229,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 39820,
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
          "id": "060c3c4f132d653c7d7f1ecf9b285b17fe18ad21",
          "message": "docs: S1.3 drops the schema lint against PactFlow's OpenAPI document\n\nLayers 1 to 3 test against the real OSS broker: every request the client sends is compared with\ntoday's verifier's or sent exactly to the pinned broker, which validates it, and selectors are the\nuser's, passed through as written. The lint's own gaps (PactFlow's document omits fetching a pact\nand publishing results, and could only be fetched, not committed) made it the weakest layer. What\nis given up is any test of PactFlow's own behaviour; PactFlow runs the same broker beneath.\n\nThe exit no longer names the lint, and the text says what is still to build from layer 1: the run\nbehind basic auth and a bearer token, and the weekly run against the latest image.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T15:57:42+11:00",
          "tree_id": "4f60fdc29a4477775cc9a436862a090e502ef7c8",
          "url": "https://github.com/rholshausen/pact-janus/commit/060c3c4f132d653c7d7f1ecf9b285b17fe18ad21"
        },
        "date": 1792386205114,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 29353,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21693,
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
          "id": "a534cd50ddce11104fb6f8dd37a3816ec6468fa4",
          "message": "test: S1.3 a broker credential reaches the broker and appears nowhere else\n\nADR 0032 decision 5, end to end: cli/tests/cli.rs puts the pinned broker behind a door that\ndemands basic credentials or a bearer token (the OSS broker has no bearer tokens, so the door is a\nproxy for both), runs janus publish and janus verify --publish through it at trace, in text and\nJSON, and with a wrong credential, and fails if the secret or the Authorization header it became\nappears in anything printed. CI's broker step runs it.\n\nIt found two leaks, neither released:\n\n- the engine's trace log of each frame repeated the broker descriptor's auth; a logged frame now\n  keeps only an auth member's kind, and a url loses its user information;\n- the HTTP library's trace output dumps raw request bytes, Authorization included; janus and\n  janus-engine hold ureq_proto at debug, where headers are redacted, whatever is asked for.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T16:09:14+11:00",
          "tree_id": "18a513f78373ed5d21e87ddea39eac0cdea41293",
          "url": "https://github.com/rholshausen/pact-janus/commit/a534cd50ddce11104fb6f8dd37a3816ec6468fa4"
        },
        "date": 1792386818659,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 25892,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21378,
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
          "id": "9317b79cb3da763b95b8af99de00d9fc0d6d9173",
          "message": "feat: S1.3 is done: the broker checks run weekly against the latest Pact Broker\n\nThe Latest Pact Broker workflow runs every check CI runs against the pinned broker against\npactfoundation/pact-broker:latest, weekly and on demand: the differential harness (--any-version),\nthe live publish, verify and credentials tests, the client's pact verified by the broker, and the\nrecording made again and compared except for its version line. Every check runs even when one\nbefore it failed, and the run's summary names the broker's version and digest beside the pin.\nNothing it does changes the repository: moving the pin stays a deliberate change.\n\npact-broker.sh takes PACT_BROKER_IMAGE and PACT_BROKER_CONTAINER, so another image can run beside\nthe pinned one, and the recording test takes JANUS_PACT_BROKER_ANY_VERSION=1. Rehearsed locally\nagainst a second broker; today the latest image is the pinned one.\n\nS1.3's Done line records what its exit did not foresee and what was left out.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-19T16:23:03+11:00",
          "tree_id": "0c1ff57df891723e91f2aad6b4b2c2d87279c92b",
          "url": "https://github.com/rholshausen/pact-janus/commit/9317b79cb3da763b95b8af99de00d9fc0d6d9173"
        },
        "date": 1792387630128,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 37798,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 29999,
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
          "id": "3302d687e7dd764544b8ecb4c9439cc391ac87cb",
          "message": "fix: the rename rehearsal rewraps the CLI's help and renames recorded contracts\n\nThe weekly rehearsal failed on two tests after S1.3.\n\nThe CLI's help is wrapped to a width, so a name of another length breaks its\nlines at other words and the trycmd snapshots no longer match. The rehearsal\nnow rewrites them from the renamed CLI and fails unless they read word for word\nas the renamed snapshots did; Documentation/codename.md adds the same step to\nrenaming for real.\n\nThe broker client's recording held each published contract as the base64 the\nbroker's API takes, so the rename could not reach the codename in its\n$format. The recording now holds the contract decoded, re-recorded against the\npinned broker.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T09:26:56+11:00",
          "tree_id": "9694828db237c187314688ce8a046001e8e69605",
          "url": "https://github.com/rholshausen/pact-janus/commit/3302d687e7dd764544b8ecb4c9439cc391ac87cb"
        },
        "date": 1792449640181,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28215,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21506,
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
          "id": "9a95033c168eddc13908d1c03920c73e345cf93b",
          "message": "feat: S1.3 verify's broker credentials also answer to --broker-*, and verify has short flags\n\n--user, --password and --token keep today's verifier spelling (S1.13), with\n--broker-user, --broker-password and --broker-token as visible aliases, so the help\nsays they are the broker's credentials and not the provider's. --log-level gets -l\n(today's verifier's short flag), verify's --provider-url -u and --output -o, and a\nbare `janus verify` prints its help, exit 2.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T10:32:33+11:00",
          "tree_id": "2af4533108b9b42353930c9d06a3f3910da97b72",
          "url": "https://github.com/rholshausen/pact-janus/commit/9a95033c168eddc13908d1c03920c73e345cf93b"
        },
        "date": 1792453021944,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 36569,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 30342,
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
          "id": "0d958a95297e857f9bd7b92e1e9c75b7172b6e96",
          "message": "feat: every janus command that takes arguments prints its help when given none\n\nclap does not propagate arg_required_else_help from the top-level command, so\ncheck, explain, upgrade, publish, component push, component pull and completions\neach set it, as verify does. The exit code stays 2. usage.md pins each bare\ncommand's first line; help.md already pins the text whole.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T10:36:07+11:00",
          "tree_id": "8585e801b33b92d7674b41aea07d2f05db703ba7",
          "url": "https://github.com/rholshausen/pact-janus/commit/0d958a95297e857f9bd7b92e1e9c75b7172b6e96"
        },
        "date": 1792453256669,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 39817,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 28211,
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
          "id": "80c4c55a76eab95cc5065f5c5dd18f8327da6b01",
          "message": "fix: S1.3 the HTTP library's logging is capped at debug, not raised to it\n\nwithout_credentials added `ureq_proto=debug` to the EnvFilter, and a directive\nsets a target's level, so at --log-level warn every request logged its debug\nlines. The cap is now a second filter an event must also pass: the HTTP library\nlogs at the level asked for, and never at trace, where it dumps request bytes.\nBoth binaries install the one subscriber in cli/src/logging.rs, and its test\npins the levels.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T10:42:59+11:00",
          "tree_id": "8b1595c86e71ebda1a7717f7c7b646f29fcda553",
          "url": "https://github.com/rholshausen/pact-janus/commit/80c4c55a76eab95cc5065f5c5dd18f8327da6b01"
        },
        "date": 1792453608437,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 40023,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 30120,
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
          "id": "7135a2c76676e54b7566fd4c21347b58c610c627",
          "message": "feat: explain and verify --explain-failures colour the plan on a terminal\n\nPlan-grammar spec §3.3 (new): a host may style either text form on a terminal,\nso long as removing the styling leaves the form exactly; nothing recorded or read\nby a program is styled, and the engine never styles.\n\nThe kernel reads a rendered plan back as tokens (plan::tokens), beside the\nrenderer; a test reads every corpus snapshot with it, nothing lost and nothing\nunrecognised. The CLI styles the tokens (src/highlight.rs) and writes them\nthrough anstream, already in the tree behind clap: colour on a terminal unless\nNO_COLOR or TERM=dumb, anywhere with CLICOLOR_FORCE. The CLI's tests now run\nwithout colour whatever CLICOLOR_FORCE says, which also fixes help.md's\nsnapshots when it is set.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T11:07:58+11:00",
          "tree_id": "7e4a4ff4ca37d36d7bcfd70713dd0f2679d0c748",
          "url": "https://github.com/rholshausen/pact-janus/commit/7135a2c76676e54b7566fd4c21347b58c610c627"
        },
        "date": 1792455155285,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28172,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21486,
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
          "id": "0f33682e29bee578028e7d6385b50767054ce136",
          "message": "fix: the broker recording normalises the engine version in the broker's echo of a result\n\nThe recording wrote the engine's version as {engine-version} in the\nverification result the client publishes, but not in the broker's 201\nanswer, which echoes verifiedBy back. A release moving the version\ntherefore changed the recording, and CI's re-record failed on the\nrelease PR. Response bodies now get the same normalisation.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T11:50:53+11:00",
          "tree_id": "de3eabcb4f4f53a850182539c82b2ee342414dc8",
          "url": "https://github.com/rholshausen/pact-janus/commit/0f33682e29bee578028e7d6385b50767054ce136"
        },
        "date": 1792458344364,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28538,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21765,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "79f09d6af69b2a1f0044e77474fc5f38490aa6ed",
          "message": "chore: release 0.2.0",
          "timestamp": "2026-10-20T12:21:26+11:00",
          "tree_id": "362620dfa584c8ad505abb2dbf71c4f5ef01d478",
          "url": "https://github.com/rholshausen/pact-janus/commit/79f09d6af69b2a1f0044e77474fc5f38490aa6ed"
        },
        "date": 1792459559232,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 31006,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 22932,
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
          "id": "ced015bd006e654d93bc09dfa45907cc7fdedf57",
          "message": "fix: the Janus benchmarks' lockfile records component-http's tracing dependency\n\nbenchmarks/janus is its own workspace with its own Cargo.lock, which bb162a0\ndid not update, so the Benchmarks workflow's --locked build failed.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T13:41:18+11:00",
          "tree_id": "c2375c84ad4b885f732b108997426d2878b59300",
          "url": "https://github.com/rholshausen/pact-janus/commit/ced015bd006e654d93bc09dfa45907cc7fdedf57"
        },
        "date": 1792464980432,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28333,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21073,
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
          "id": "6ca0f39929209673c5ce93f226b454caea18eb74",
          "message": "fix: a contract's executed plan shows the response it was scored on\n\nA shape-compiled plan carries both parts, but verification resolves it against\nthe provider's reply alone, so the request half of the executed plan resolved\nevery path to absence and printed as failing. Only the response is scored, so\nthe verification/executed-plan event's text is now that subtree: the same as a\nv1–v4 pact's, and the plan that decided the verdict.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T15:21:40+11:00",
          "tree_id": "ec7bead8a2e9776ea8d82a4dd606f77d48b62264",
          "url": "https://github.com/rholshausen/pact-janus/commit/6ca0f39929209673c5ce93f226b454caea18eb74"
        },
        "date": 1792470365273,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 27539,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21341,
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
          "id": "5db014a1a66d0ab9a7be1220bf817209f4b04e0c",
          "message": "feat: S1.4 a v1–v4 pact's request generators are applied in verification\n\nADR 0034. The v1–v4 generator vocabulary is the generator component `pact`,\nbuilt into the kernel behind generator/generate and producing values with\npact_models, as today's verifier does. Verifying a pact generates the request's\npath, headers, query and JSON body after state setup and before before-request;\nresponse generators are not applied, as today.\n\nA state-setup hook result carries `values`, and the pact-state-change format\nreads a JSON object reply as them, so a state endpoint that returns {\"id\": 1234}\nfeeds ${id} unchanged. A generator that produces nothing keeps the recorded\nvalue, as today, and the run emits a generator-failed verification/warning.\n\nS1.1's corpus gains four generators/ fixtures (pact-reference's issue #441 case,\na state-change reply into a path and header, every value generator from the\ncompatibility suite, and an expression with nothing to read); the harness's\nroutes gain `patterns` and its state callback a `values` answer. All agree with\ntoday's verifier; janus from before this disagreed on all but the last.\n\nSpecs: lifecycle-hooks §3.2, §4.4, §8.4, §10.2; component-interfaces §7.3 and\n§11.1 (unknown-generator, generate-failed); the verification/warning event,\nemitted since the prototype, now in the protocol's event table and schema, with\nthe bindings regenerated. Generators in contracts are the new item S1.19.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-20T16:33:02+11:00",
          "tree_id": "f9cb2b823af26259e7c5031b2521e8165178036a",
          "url": "https://github.com/rholshausen/pact-janus/commit/5db014a1a66d0ab9a7be1220bf817209f4b04e0c"
        },
        "date": 1792475188195,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28390,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21584,
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
          "id": "ca5162bc2013ce0e535fcab4ee8116079b4a82e2",
          "message": "feat: S1.5 providers and brokers are reached through one async HTTP client\n\nADR 0035. engine/http-client is the one client the HTTP transport's drive role\nand the broker client send through: reqwest on rustls with ring and the platform\nverifier, today's verifier's stack, already in the tree through pact_models. It\nruns on a current-thread runtime with names resolved inside the request, so it\nstarts no thread; a Linux test counts the process's threads around a request.\n\nOne document configures it, the HTTP client options (component-interfaces spec\n§5.6, http-transport.schema.json): TLS roots, a client certificate and an\ninsecure mode; proxies (the environment's by default, one named, or none);\nrequest and connect timeouts; HTTP/1.1, HTTP/2 or negotiated; retries. It is the\ndrive options' `client` and the broker descriptor's. The drive options add\n`base-url`, whose path is the base path, and `headers`, which replace the\ninteraction's own of the same name in any case. Certificates and keys are PEM\ntext; the frame trace log hides a key and every provider header value.\n\njanus verify takes today's --base-path, --header, --request-timeout and\n--disable-ssl-verification, with their environment variables, and adds\n--ca-cert, --client-cert/--client-key, --proxy, --no-proxy, --connect-timeout\nand --http-version. Documentation/verifier-migration.md is S1.13's table, with\nevery transport option of both of today's verifiers filled in.\n\nBreaking: verification's sends carry no timeout-ms, so a provider request is\nbounded by the transport's request timeout, 5 s by default as today, where the\nkernel's was 30 s. A variant whose request could not be sent now carries the\nprotocol's component-failed error, with a message saying why.\n\nThe exit's verification is in cli/tests/cli.rs: a TLS provider demanding a\nclient certificate, through a proxy, at trace, printing neither the key nor the\nheader's token. tools/test-servers holds the authority, TLS provider and proxy\nthe tests share. S1.1's corpus gains six connection/ fixtures; five agree with\ntoday's verifier, and connection/header-case is a deliberate difference in the\nledger. The whole corpus changed no verdict.\n\nFound: the prototype's client asked providers for gzip and trusted its own\nbundled roots, both now as today's verifier. The hooks' trace-log redaction gap\nand the http hook's client are added to S1.9.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T10:56:06+11:00",
          "tree_id": "3535aae04eccead1522f2807fa684102e705db69",
          "url": "https://github.com/rholshausen/pact-janus/commit/ca5162bc2013ce0e535fcab4ee8116079b4a82e2"
        },
        "date": 1792540998785,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 40517,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 28497,
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
          "id": "8f2789c0c390af20d98972eb841056127336dde9",
          "message": "docs: split S1.6 into a body kind per item, S1.6 and S1.20 to S1.24\n\nS1.6 was every body kind at once (L). It is now plain-text and binary bodies\n(S), the two that need nothing new in the document model. Member order and the\nlegacy body compiler's retyping, which form and XML both need, become S1.20,\nafter S1.11. Form-urlencoded and XML follow as S1.21 and S1.22, and multipart,\nwhich compares each part through the others' components, comes last as S1.23.\nEach closes its own S1.1 ledger entries, so each can be marked done on its own.\n\nMeasured first: of the 158 skipped pact-specification cases, 156 are XML, and\ntwo carry a janus:skip blaming unanchored regexes, which S1.18 anchored; both\npass without it, and S1.6 removes the markers. So the \"no janus:skip left\" exit\nmoves to S1.22. The four matching rules the legacy compiler refuses (Values,\nEachKey, EachValue, ArrayContains) were in S1.6's findings row though no case\ncovers them; they are S1.24.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T11:05:31+11:00",
          "tree_id": "cb9c99ff5af450259a97322c11fe5b00ef19b83d",
          "url": "https://github.com/rholshausen/pact-janus/commit/8f2789c0c390af20d98972eb841056127336dde9"
        },
        "date": 1792541436073,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 27354,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 20130,
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
          "id": "28e86c853aa12361c481a1da856cc62409dafadd",
          "message": "fix: the SDK bindings carry S1.5's schema descriptions\n\nS1.5 reworded the broker descriptor's `client` description and described a\ntransport binding's `options`, pointing both at the HTTP client options. The\ngenerated TypeScript and JVM bindings carry descriptions as doc comments, and\nwere not regenerated, so CI's \"Bindings match their schemas\" step failed.\nRegenerated with `cargo run -p pact_janus_bindings -- generate`; doc comments\nonly.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T11:24:12+11:00",
          "tree_id": "372773487eefbbd740a43dbb933fe6d25d8a58c8",
          "url": "https://github.com/rholshausen/pact-janus/commit/28e86c853aa12361c481a1da856cc62409dafadd"
        },
        "date": 1792543143005,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 27928,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21143,
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
          "id": "3412e5c41fd6ad192af6a7b784f11bba769d3aeb",
          "message": "fix: a request the provider could not answer is reported once, on one line\n\njanus verify printed a variant's component-failed error with its details as a\nJSON line beneath it, and the engine logged the same failure again at warn, in\nRust's debug format. The line now ends with the component's own code in\nbrackets, which is all the JSON added; --output json still carries the whole\nerror. The engine logs a failed variant, a transport failure or mismatches, at\ndebug, since the run's result event reports it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T11:40:33+11:00",
          "tree_id": "0b9538c84e3b3a88efabffefe1133de1f624d152",
          "url": "https://github.com/rholshausen/pact-janus/commit/3412e5c41fd6ad192af6a7b784f11bba769d3aeb"
        },
        "date": 1792543466851,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 40220,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 29663,
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
          "id": "379f7ff5398a4899c9464109b13829ed677ccd36",
          "message": "refactor: S1.11 the v1–v4 compatibility layer is the kernel's legacy module\n\nKernel-boundary review findings 3 and 6: nothing in the module tree marked the\nv1–v4 pact reader and matching-rule compiler as the permanent HTTP- and\nJSON-shaped exception to \"the kernel knows nothing about HTTP or JSON\".\n\nengine/kernel/src/legacy/ is that boundary, and its mod.rs says so: pact (was\nlegacy_pact.rs), compile (was plan/legacy.rs) and upgrade (was upgrade.rs,\nwhich needs the same knowledge). A module, not a crate: the compiler builds the\nkernel's plan nodes and verification calls the compiler, so each would need the\nother.\n\nengine/kernel/tests/boundary.rs holds the rest of the kernel to it, in cargo\ntest on every system: outside legacy/, any pact_models use fails unless its\nlist allows it, and it allows two, neither HTTP nor a pact document (the\nbuilt-in `pact` generator component's vocabulary, and the datetime operator's\nformat check). S1.4 had put a pact_models request in the verification path for\nits generators; it is now an opaque legacy::pact::RequestTemplate.\n\nAGENTS.md's architecture rule names the exception. No behaviour changes.\n\nNo-Changeset: a move inside the kernel; nothing an engine pin sees changes.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T12:05:35+11:00",
          "tree_id": "c66c1525b5116af4af30f88c5757ce6403cd3578",
          "url": "https://github.com/rholshausen/pact-janus/commit/379f7ff5398a4899c9464109b13829ed677ccd36"
        },
        "date": 1792545011935,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28336,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 20140,
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
          "id": "e3cf39ba68bc60027de7f29f0c8f1df749b857dc",
          "message": "feat: S1.6 text and binary bodies verify, through a text component and bytes\n\nADR 0036. A v1–v4 pact whose body was plain text failed as\ncomponent-unavailable, and a binary one as contract-invalid; today's verifier\npasses both.\n\n- engine/component-text is a built-in content component for text/*: it decodes\n  UTF-8 octets to a string and encodes a string back, whatever the charset\n  says. Every embedding registers it after JSON, and engine/hello lists it.\n- A content slot no component claims now decodes to bytes rather than absent.\n  One a component claims and cannot decode is still absent.\n- The legacy layer reads a body by the pact's content type, as today's\n  verifier picks a body matcher: JSON as a document, a binary type as bytes,\n  other text as a string (bytes when not UTF-8), and XML, form and multipart\n  refused until S1.21 to S1.23. A text or bytes body compiles to one value at\n  $: its rules, or match:equality.\n- A reply's body is decoded as the pact's type, and a new legacy action,\n  legacy:body-type, fails a reply whose content type disagrees, with today's\n  message. It is emitted only when the pact names no Content-Type header, so\n  six legacy corpus snapshots show it now; their results are unchanged.\n- A bytes mismatch names the first byte that differs.\n- A corpus case may carry bytes through an optional `encoded` map\n  (plan-grammar spec §6.1). New corpus cases cover text, regex text and\n  binary bodies; corpora/verifier/bodies/ adds the harness fixtures, and the\n  four S1.6 entries leave the disagreement ledger.\n\nThe plan gains S1.25 (janus upgrade converts text and binary bodies) and S6.5\n(charsets other than UTF-8, in the text component).\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T15:47:40+11:00",
          "tree_id": "11561b2d312220e3177a2f2511ddd861e90f39f4",
          "url": "https://github.com/rholshausen/pact-janus/commit/e3cf39ba68bc60027de7f29f0c8f1df749b857dc"
        },
        "date": 1792558927048,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28298,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21616,
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
          "id": "a862fdb4818e876cdadc8ec08a8229606af26d83",
          "message": "fix: a bytes value in a plan shows its length and base64\n\nThe text forms printed a bytes value as `<8 byte(s)>`, which said nothing\nabout the bytes, and `<...>` already means `<absent>` in mismatch messages.\nThey now print `BYTES(8, iVBORw0KGgo=)`, as the forked pact_matching engine\ndid: the length, then the base64, which is how a v1–v4 pact and the protocol\ncarry the bytes. The base64 is cut after 48 characters (the first 36 octets,\non a 4-character boundary) and ends in `...`, which no base64 alphabet holds.\n\nOne function in plan/render.rs prints bytes for the pretty and executed forms\nand for mismatch messages; the highlighter's lexer reads the new form as a\nliteral. Plan-grammar spec §3.1 states the form, and the two binary-body\ncorpus cases are re-accepted with it.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T17:20:49+11:00",
          "tree_id": "649d863fc9927a70be678ebb688be341f1e08ce0",
          "url": "https://github.com/rholshausen/pact-janus/commit/a862fdb4818e876cdadc8ec08a8229606af26d83"
        },
        "date": 1792563931182,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 28564,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21572,
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
          "id": "816e4b7c8c22e9e0b0412419ac6ef182cff22dc6",
          "message": "fix: the built-in text component serves only a body that is a string\n\nS1.6's text component claims text/*, so a text/csv body of rows with no CSV\ncomponent declared stopped failing as component-unavailable: the content-type\ncheck found text, and the mock would only have failed later, with\nencode-failed, when it tried to write the rows. The TypeScript SDK's CSV\nconsumer test caught it in CI.\n\nThe text component's compile now refuses a shape that admits no string\n(objects, arrays, each-like, numbers, booleans, and the choices and wrappers\nof only those). The engine asks the in-tree component that handles a declared\nslot to compile its shape when the interaction arrives and before a\nverification starts, as it already asked declared components, and a failure\nis component-unavailable with reason compile-failed, naming content/<type>.\nScope::check now runs that and the declared components' fragment check\ntogether. A text/ body that is one string is still served as text with no\ncomponent declared; v1–v4 pacts do not go through this check.\n\nADR 0036 decision 1 and component-interfaces spec §6.3 say so, and the\nunreleased S1.6 change file is updated to match.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-21T17:34:07+11:00",
          "tree_id": "02f2865b3390ce4c44e7589a77ff22af9ae11e08",
          "url": "https://github.com/rholshausen/pact-janus/commit/816e4b7c8c22e9e0b0412419ac6ef182cff22dc6"
        },
        "date": 1792565286646,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 36090,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 29781,
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
          "id": "8a8449e082b0e74549f10c83899393fdd89d8773",
          "message": "feat: S1.20 member order is carried through the engine, and decided by nothing\n\nADR 0037. The engine sorted every object's members, so a mock served a CSV\nbody's columns, or a JSON body's members, alphabetically whatever order the\nconsumer declared them in (phase-9 finding 11).\n\n- serde_json's preserve_order is on, the runtime value's objects and an\n  object shape's members are insertion-ordered, and a generated document\n  follows the declared order, so content/encode sees the author's order.\n- Order is never compared: objects with the same members are equal in any\n  order, and plans, the variant space, subsumption and the v1-v4 compiler\n  walk members by name. Two places had begun reading order once it was\n  kept and now go by name: subsumption's same-admits test, which zipped two\n  members maps, and janus upgrade's each-entry sample.\n- The canonical writer had relied on sorted maps. Records and a shape's own\n  members are written by name; an object shape's members, examples, options,\n  recorded content and state parameter values keep their order\n  (contract-file spec §2.4). Provider shapes follow the same rule. A contract\n  whose members were in name order writes the same bytes as before.\n- The legacy compiler compiles a body as a document (LegacyBody::Document of\n  a RuntimeValue), not serde_json::Value (kernel-boundary review finding 5),\n  so form and XML components can hand it what they decode.\n- ComponentError and EngineError box their details, since an ordered Value\n  is larger; the protocol is unchanged.\n\nNo plan or result in corpora/ moved; five executed.txt snapshots print a\ncaptured body in the order the case wrote it. A new case,\nshapes/member-order, compares an example and a body holding the same members\nin other orders. S1.1's harness: 326 verdicts, 312 agree, 14 known. The CSV\nhost test and the TypeScript SDK's CSV consumer check the column order.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T11:07:20+11:00",
          "tree_id": "e0953a1a18802820d0a531e8f1af65fe4ae8e62a",
          "url": "https://github.com/rholshausen/pact-janus/commit/8a8449e082b0e74549f10c83899393fdd89d8773"
        },
        "date": 1792627976223,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 39059,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 27148,
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
          "id": "648b4dc90df51ab7cfa44e8c19f0290507d98c59",
          "message": "fix: the broker client's recording keeps the order the broker sends\n\nCI's verifier-diff job records the broker scenario again and fails on any\ndifference. Since S1.20 (ADR 0037) a Value keeps its members in the order\nthey arrived, so the recorder writes the broker's replies and our requests\nin their own order rather than sorted. The content is unchanged, the 409 the\nscenario provokes included; only member order moved. Recorded again against\nthe pinned broker, and the result is byte-identical across two fresh\nbrokers.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T12:06:08+11:00",
          "tree_id": "2d7e6b70ea0f8e75ea828f14660cfb394445ea07",
          "url": "https://github.com/rholshausen/pact-janus/commit/648b4dc90df51ab7cfa44e8c19f0290507d98c59"
        },
        "date": 1792631510609,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 26976,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 20445,
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
          "id": "8aaf6091a572b0a41bc17a7d49e8ca733377f414",
          "message": "docs: cite pact-reference#588 for today's unapplied form body generators\n\nThe ledger's two generators/form-body entries name the upstream issue:\npact_verifier builds pact_matching without its form_urlencoded feature, so\ntoday's verifier sends a form body's generators unapplied. ADR 0038 said\ntoday's verifier generates into a form body; it now says pact_matching does\nwhen built to, pact_verifier is not, and pact-jvm does, citing the issue.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T12:15:33+11:00",
          "tree_id": "a3a786f54eb917e612cdceb90f07f528e2d22b31",
          "url": "https://github.com/rholshausen/pact-janus/commit/8aaf6091a572b0a41bc17a7d49e8ca733377f414"
        },
        "date": 1792632069716,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 37746,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 27947,
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
          "id": "1dbc1d278e2ed5d00f929cdd3546194cf819594e",
          "message": "chore: the benchmark chart draws a trend line, the median of the last runs\n\nThe runners are shared machines, so a single run's numbers move by tens of\npercent and a change in the code is hard to see on the chart. The chart page\nnow draws, at each run, the median of the last 7 runs over the runs\nthemselves, which are faded; ?window=N on the address changes how many.\n\nThe page was github-action-benchmark's default, which it writes to gh-pages\nonly when there is none. Its source now lives in benchmarks/chart/index.html,\nand the Benchmarks workflow publishes it as bench/index.html after each\nrecorded run, committing only when it differs.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T12:40:35+11:00",
          "tree_id": "9956137246a351b5bf1dd3dacb31d6a210472a63",
          "url": "https://github.com/rholshausen/pact-janus/commit/1dbc1d278e2ed5d00f929cdd3546194cf819594e"
        },
        "date": 1792633538313,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 26901,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 20148,
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
          "id": "bc39a46924ad38f93319780dbc31296e1b10da52",
          "message": "feat: S1.22 XML bodies verify as today, through a built-in XML component\n\nA v1–v4 pact with an XML body was refused, and the legacy harness skipped\nthe 156 pact-specification cases that have one. ADR 0039 decides how XML\nis read and compared.\n\nengine/component-xml is a built-in content component for the types today's\nverifier counts as XML (application or text, subtype xml or +xml). Every\nembedding registers it after JSON and before text, so text/xml reaches it.\nIt parses with sxd-document, as today's verifier does. Its document is\nkeyed by element name: an element holds #namespace, @<name> per attribute,\n#text (each text node trimmed, then joined) and an array of children per\nlocal name, so pact's XML paths are document paths.\n\nThe legacy layer reads a pact's XML body into the same document, and a test\nholds that reader to the component's. The legacy compiler walks it as\npact_matching's match_xml does, with three new legacy actions:\nlegacy:xml-children, legacy:xml-only-children and legacy:parses-as. Plan\npaths gain a quoted segment, ['@{urn:names}name']. Request generators on an\nXML body go through pact_models' XmlHandler.\n\nA type rule on any v1–v4 text value (attribute, element text, path, header,\nquery or form value) now parses the text, as today's verifier does, where\nit compared the value's kind.\n\nThe legacy harness runs all 741 cases, with no exclusion rule and no\njanus:skip. The differential harness reads 344 verdicts: 330 agree and 14\nare known. Its ledger entries for compatibility-suite case 19 are gone. One\nnew entry is deliberate: an element the pact puts in no namespace matches\nits local name in any namespace, as the specification's `unexpected xml\nnamespace` case says and pact-reference's V2 engine does, where today's\nverifier and pact-jvm fail it.\n\nThe staged plan adds S2.10: keeping XML text as written, CDATA included,\nin Janus contracts only.\n\nThe XML walk follows pact_matching::xml (pact-reference, rust/pact_matching\n2.0.11); no code was copied.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T16:18:29+11:00",
          "tree_id": "d6d6727384e3368ffa58a5d037dd75287f00123d",
          "url": "https://github.com/rholshausen/pact-janus/commit/bc39a46924ad38f93319780dbc31296e1b10da52"
        },
        "date": 1792646616028,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 25564,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 20318,
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
          "id": "f2da18e712b02bdcf2cc6df6623284295781fb5f",
          "message": "docs: the v1–v4 compiler may emit expect:absent for a body declared empty\n\nPlan-grammar §4.4 and §4.5 said the v1–v4 compiler never emits\nexpect:absent, since v1–v4 has no absence assertion. It has one: a body\nthe pact declares empty or null fails when the actual body has content\n(\"Expected empty body but received …\", pact_matching 2.0.11\nmatch_body_content) and passes when it is missing or empty. The legacy\ncompiler has emitted expect:absent on the body slot for exactly that since\nthe prototype, and the specification case `body/non empty body found when\nempty expected` holds it to today's verdict.\n\nThe spec now allows expect:absent there and nowhere else in a v1–v4 plan;\nmatch:any-of stays shapes only. No behaviour or corpus changes. Found while\nlanding S1.22.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T16:22:57+11:00",
          "tree_id": "25a0f07018c6aa287d7a56a7cdf90f2d94948293",
          "url": "https://github.com/rholshausen/pact-janus/commit/f2da18e712b02bdcf2cc6df6623284295781fb5f"
        },
        "date": 1792646944621,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 27164,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 19605,
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
          "id": "ba66863adea760556b544dbea6229fd262edb70e",
          "message": "feat: S1.23 multipart bodies verify as today, through a built-in multipart component\n\nA v1–v4 pact with a multipart body was refused as component-unavailable, the\nlast body kind the legacy layer refused. ADR 0040 decides how multipart is\nread and compared.\n\nengine/component-multipart claims every multipart/* type and is registered\nafter form in every embedding. It splits a body with multer, as today's\nverifier does, at the boundary its content type names (read with mime, as\ntoday; multer's own parse_boundary reads only multipart/form-data). Its\ndocument is keyed by part name, unnamed parts under \"\", and a part is\nheaders, filename and body. Each part's body is decoded through the built-in\nJSON, XML, text and form components the multipart component holds itself;\nany other part is bytes. Reaching declared components is the new S5.6.\n\nThe legacy layer splits a pact's body the same way and reads each part by\nits own type, and a test holds that reader to the component's over every\nmultipart body in the repository. The legacy compiler compares the parts as\npact_matching's match_mime_multipart does: by name, or by position among the\nunnamed parts; a new legacy action, legacy:mime-part, fails a missing part\nor the wrong kind with today's messages; then each part's headers, and its\nbody, a text field as text and a file by its own type with the rules under\n$.<part> re-rooted. A reply is decoded under its own label, because its\nboundary is its own. Unsupported::ContentType is gone: no body kind is\nrefused.\n\nThe differential harness reads 399 verdicts over 183 cases: 389 agree and 10\nare known. The four ledger entries for compatibility-suite case 25 are gone.\nbodies/every-body-kind, one v3 pact with XML, form, multipart and binary\nbodies (S1.6's combined exit), and bodies/multipart-form-data agree with\ntoday's verifier, mutations included.\n\nThe multipart walk follows pact_matching::binary_utils (pact-reference,\nrust/pact_matching 2.0.11); no code was copied.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T17:39:22+11:00",
          "tree_id": "17de75f266163bbf0117d8fe9da1636793d70f63",
          "url": "https://github.com/rholshausen/pact-janus/commit/ba66863adea760556b544dbea6229fd262edb70e"
        },
        "date": 1792651424000,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 64044,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 45175,
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
          "id": "37282376c5d1521d5a6a11b62220eceb12c8dca7",
          "message": "test: record today's second Content-Type rule check as S1.26\n\nFound landing S1.23. Today's verifier checks a rule on the pact's\nContent-Type header on the header as sent, and again before it compares a\nbody, against the reply's type as pact_models writes it: no space before a\nparameter, charset in lower case. A regex written with the space passes the\nfirst check and fails the second. janus verify checks the rule once, on the\nheader (ADR 0036 decision 4), and passes.\n\nThe fixture bodies/content-type-rule-with-a-space shows it on a JSON body,\nso it is not multipart's alone, and its two ledger entries name S1.26, a new\nplan item that decides whether the legacy compiler copies the second check\nor an ADR makes the difference deliberate. Nothing the engine ships changes.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T17:44:22+11:00",
          "tree_id": "789ac731a096dd03b98419ef52cbcf8dd49db9aa",
          "url": "https://github.com/rholshausen/pact-janus/commit/37282376c5d1521d5a6a11b62220eceb12c8dca7"
        },
        "date": 1792651763472,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 37381,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 27519,
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
          "id": "4057e1433a8668942599a644f6ff9971c9633e9d",
          "message": "fix: generated bindings keep the schema's member order however the generator is built\n\nThe release builds janus-sdk-gates alongside the CLI, so Cargo unified the\nkernel's serde_json `preserve_order` (ADR 0037, S1.20) into the bindings\ngenerator: the release's gates wrote members in schema order while\n`cargo run -p pact_janus_bindings` sorted them, and the release's bindings\ngate failed on both SDKs. The bindings crate now asks for `preserve_order`\nitself, and the checked-in bindings are regenerated in schema order.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-22T18:20:05+11:00",
          "tree_id": "0ead9bb71c37035b34eee234b3e2c6e86df4fbeb",
          "url": "https://github.com/rholshausen/pact-janus/commit/4057e1433a8668942599a644f6ff9971c9633e9d"
        },
        "date": 1792654465998,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 30311,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 21509,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "38309842+rholshausen@users.noreply.github.com",
            "name": "Ronald Holshausen",
            "username": "rholshausen"
          },
          "distinct": true,
          "id": "1bb2c01bf3c316eba4bd5a3f771000530a96d1f1",
          "message": "chore: release 0.3.0",
          "timestamp": "2026-10-22T18:49:44+11:00",
          "tree_id": "20eda7a8696566c70e416d916648bd143ae8d221",
          "url": "https://github.com/rholshausen/pact-janus/commit/1bb2c01bf3c316eba4bd5a3f771000530a96d1f1"
        },
        "date": 1792656238296,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 27355,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 20486,
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
          "id": "7de3724e7af2d45f501bfd9884afc48338adeb67",
          "message": "feat: S1.17 a filtered run is reported as filtered, never verified, and is not published\n\nA --variant naming an id no document recorded replayed nothing and reported\nVERIFIED with exit 0; from a broker with --publish it published a success\nfor every pact, since a pact with no results had none that failed. ADR 0041\ndecides what a filtered run is and how it reads:\n\n- A filter naming an id no document records is variant-not-found, with the\n  first such id and the ids the documents record. verify refuses it for an\n  inline source (janus verify exits 2); a broker run ends right after\n  verification/fetched, before anything runs or is published. An empty\n  variants list is malformed-frame.\n- A run is filtered when its filter left out a recorded variant, as pact-jvm\n  judges it; one that admits everything (base, over v1-v4 pacts) is whole.\n- A filtered run that nothing failed has status filtered, and so does each\n  pairs entry it narrowed. janus verify prints FILTERED and exits 0: it did\n  what it was asked, as today's verifiers do.\n- A narrowed pact is not published: outcome skipped, reason filtered, as\n  pact-jvm skips it. pact_verifier_cli 1.3.5 publishes it, and passes a\n  filter that matches nothing with exit 0.\n- subsumption/decide raises verification-filtered per pair.\n\nThe variant-semantics and engine-protocol specs say so. The cases are\nprotocol-level tests in the kernel's verification and broker tests, and two\nCLI tests replace the snapshot that recorded the gap; corpora/ holds plans\nand conformance/ has no provider side until S1.14, which gains them. S1.13's\n--filter-* rows carry ADR 0041.\n\nAlso: S1.18's Done line in the staged plan (178cb64), and start()'s doc\ncomment, which had drifted onto RunComponents.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T10:24:25+11:00",
          "tree_id": "4cf962543227f55092ed4346084d123eae52adfc",
          "url": "https://github.com/rholshausen/pact-janus/commit/7de3724e7af2d45f501bfd9884afc48338adeb67"
        },
        "date": 1792712404940,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 39011,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 27812,
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
          "id": "2f4150297cc97156526009a32fed5c6e26ce0739",
          "message": "feat: S1.24 values, eachKey, eachValue and arrayContains verify as today\n\nA v1-v4 pact whose body used one of these four rules failed every\ninteraction that had one: the legacy compiler compiled each to flow:error.\nNo pact-specification case covers them. ADR 0042 compiles them as\npact_matching's compare_maps_with_matchingrule and\ncompare_lists_with_matchingrule apply them, on a body's objects and arrays:\n\n- values, eachValue and arrayContains on an object are a flow:for-each over\n  its entries, dispatching on ~>.key: each entry against the pact's value\n  for its key, or the pact's first value by key; keys are not compared.\n  eachValue swaps in its own rules at <path>.*. Branches are labelled as the\n  member, or {*} for any other key.\n- eachKey holds every key to its rules (labelled {key}), and compares the\n  pact's own members only where the reply has them.\n- arrayContains on an array is one legacy:array-contains per variant. The\n  action was in the grammar and never emitted; it now runs a template, lets\n  one element satisfy two variants (match:contains does not), and fails\n  with today's \"Variant at index 0 (...) was not found in the actual list\".\n  Its template is labelled [?i], specified in plan-grammar spec 4.4.\n- eachValue on an array holds every element to its rules, and checks\n  nothing when cascaded. On a scalar, values is equality and the other\n  three accept any value, as today's JSON matcher does.\n\nS1.1's new matching-rules/ fixtures, one per rule with replies that match\nand that do not and mutations, give 379 verdicts over 183 cases, all\nagreeing with pact_verifier_cli 1.3.5; the whole harness reads 780\nverdicts, 768 agreeing and 12 known. Corpus cases: legacy/v3-values,\nv4-each-key, v4-each-value, v4-array-contains and their -mismatch twins.\n\nLeft out: the four declared on text (path, header, query, form, XML) are\nstill refused; janus upgrade still converts them approximately (S1.25);\nmismatches inside entry templates report relative paths (~>.value.id),\nnoted for the CLI output review.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T11:19:52+11:00",
          "tree_id": "3eea76c7387557ca880fb5cf83ae049d5c3df3fa",
          "url": "https://github.com/rholshausen/pact-janus/commit/2f4150297cc97156526009a32fed5c6e26ce0739"
        },
        "date": 1792715060705,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 62437,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 46004,
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
          "id": "ea1f4368e80ec5f43e3cdbd6fb5ed2725324ddfb",
          "message": "docs: S1.26 a pact's Content-Type is checked once, on the header (ADR 0043)\n\nToday's verifier checks a named Content-Type a second time before the body:\nwhen the pact's type has parameters, the reply must have exactly those, or\nthe header's rule must match both types as pact_models writes them (no\nspace, charset in lower case). It fails replies its own header check has\njust accepted: a rule written with a space and another charset, and,\nfound here, a reply that adds a parameter with no rule at all. pact-jvm\ncompares base types only and passes both, as janus verify does.\n\nADR 0043 makes the difference deliberate. bodies/content-type-extra-\nparameter is the new fixture, and the four ledger entries for it and\nbodies/content-type-rule-with-a-space carry the ADR. A probe of the\nneighbouring cases (charset case, a pact type with no parameters, a rule\nwith the same charset or written to allow no space) found them agreeing.\nThe harness reads 782 verdicts: 768 agree, 14 known. Nothing the engine\nships changes.\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T11:42:39+11:00",
          "tree_id": "1b036ce771d696d21e8905552a1c3f417df416f7",
          "url": "https://github.com/rholshausen/pact-janus/commit/ea1f4368e80ec5f43e3cdbd6fb5ed2725324ddfb"
        },
        "date": 1792717294885,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 57546,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 48198,
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
          "id": "31ece188f8ee180d738a072596d9b2ac3d0851f3",
          "message": "fix: the benchmark lock file follows hooks-host, and exec's tests are Unix-only on every target\n\nS1.9 changed hooks-host's dependencies without updating\nbenchmarks/janus/Cargo.lock, so --locked failed in the Benchmarks run and\nthe checks job. The exec deadline tests gated each item to Unix but not\ntheir imports, which Windows clippy refused as unused; the file is now\ngated as a whole.\n\nNo-Changeset: a lock file outside the release and a test file\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T13:23:29+11:00",
          "tree_id": "5b7c8829941a76f548a1851cc7a45b49c403b002",
          "url": "https://github.com/rholshausen/pact-janus/commit/31ece188f8ee180d738a072596d9b2ac3d0851f3"
        },
        "date": 1792722515231,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 26537,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 19534,
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
          "id": "3094b45487990cf62fb3c45e0f3fa3f0a6c5a2c1",
          "message": "feat: S1.8 an interaction that needs a pact plugin is refused by name, and the rest of its pact verifies (ADR 0045)\n\nA v1-v4 interaction needs a pact plugin when its pluginConfiguration names\none, or when it uses a rule pact_models reads as a plugin's and the pact\nnames plugins (then it needs all of them, as today's verifier loads).\n\n- janus verify refuses each such interaction before any state or request,\n  as component-unavailable naming pact-plugin/<name>; the rest of the pact\n  verifies, and the summary counts the refusal as a failure.\n- janus upgrade drops it with an interaction-dropped finding naming the\n  plugin; janus explain refuses it with the same error.\n- Previously a CSV/JWT/credit-card HTTP interaction was verified only in\n  part with the built-in components, and a gRPC/Avro message one was\n  skipped silently.\n\nNo S1.1 fixture: today's verifier installs a missing plugin from the\ninternet. The refusals are tested offline in the kernel and the CLI.\nFixtures are trimmed from pact-plugins' examples (MIT).\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
          "timestamp": "2026-10-23T16:55:17+11:00",
          "tree_id": "17b04a6ec47e8f14fb2dccd778bdf314a3bc96cf",
          "url": "https://github.com/rholshausen/pact-janus/commit/3094b45487990cf62fb3c45e0f3fa3f0a6c5a2c1"
        },
        "date": 1792735225624,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "mock-server-throughput-4engines (native)",
            "value": 25179,
            "unit": "requests per second",
            "extra": "engines 4"
          },
          {
            "name": "mock-server-throughput-4engines (subprocess)",
            "value": 20290,
            "unit": "requests per second",
            "extra": "engines 4"
          }
        ]
      }
    ]
  }
}