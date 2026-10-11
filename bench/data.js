window.BENCHMARK_DATA = {
  "lastUpdate": 1791695601966,
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
      }
    ]
  }
}