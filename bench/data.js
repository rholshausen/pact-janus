window.BENCHMARK_DATA = {
  "lastUpdate": 1791696156286,
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
      }
    ]
  }
}