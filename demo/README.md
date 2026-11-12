# The Pact Janus demo

Plan task **9.3**: the RFC's whole loop, against real code, in about a minute.

![The demo, recorded with VHS from demo.tape](demo.gif)

| Step | What happens | What it shows |
|---|---|---|
| 1 | `web-app`'s consumer test declares that an order's status is `PENDING` or `SHIPPED`, that it may have a `shippedAt`, and that it has at least one item. It excludes the one combination the provider cannot produce, a `SHIPPED` order with no `shippedAt`. The engine runs the test once per variant it selects (6) and records a contract | Variant testing: a declared variation is a promise to exercise it (RFC, "A consumer test in Pact MkII"), and an exclusion says, with a reason, which one is not (ADR 0080) |
| 2 | `janus verify` replays each variant against the running `order-service`. Its verifier config gets an OAuth2 token, sets up each variant's provider state through the provider's existing v3 state endpoint, and adds a header with a script | Verification with hooks. Provider states are bound to variants with `whenVariant` and `variantCases` (ADR 0009) |
| 3 | `janus check` compares the contract with the shape `order-service` recorded from its own tests, and adds the verification result: the provider may send `CANCELLED`, and an order with no items. It also names the exclusion, with its reason, beside the fields it left untested together | The subsumption check, which is what verification alone cannot see (RFC, "Variance the consumer never declared") |
| 4 | The consumer widens its contract to what the provider may send. The engine now selects 6 variants, and 4 of them fail the consumer's code, each named with its cause. The SDK withholds the contract | Variant testing forcing the consumer to handle what it declared |
| 5 | The consumer fixes its code, and all 6 variants pass | |
| 6 | Verification and `janus check` pass | The loop, closed |

## Running it

```sh
demo/run.sh            # press Enter between steps
demo/run.sh --pace 5   # 5 seconds between steps
demo/run.sh --ci       # no pauses; checks each step's outcome and exits non-zero if one differs (CI runs this)
```

It needs bash on Linux or macOS (it is built on symbolic links and background jobs, and CI runs it on
both), cargo, and Node 22.6 or later with npm. It builds `janus`, `janus-engine` and the sample
provider, and starts `order-service` on a free port. It packs the TypeScript SDK from this tree with
`npm pack`, which builds the same tarball a release publishes, and installs it in a scratch copy of the
web app beside Vitest, as a project installs a release. Each step's change is applied to that copy, so
nothing in the repo changes but build output.

To record the screencast again, from the repo root, with [VHS](https://github.com/charmbracelet/vhs),
`ttyd` and `ffmpeg` on the PATH:

```sh
vhs demo/demo.tape     # writes demo/demo.gif
```

VHS v0.12.0 writes no file and still exits 0. It cancels its own context before rendering, so every
ffmpeg call is killed before it starts ([charmbracelet/vhs#787](https://github.com/charmbracelet/vhs/issues/787);
fixes are open as #788–#791). Until a fixed release ships, use v0.11.0, or build v0.12.0 with
`v.Render(context.WithoutCancel(ctx))` in `evaluator.go`, which is how `demo.gif` was recorded.

## What is here

| Path | |
|---|---|
| `web-app/` | The consumer: `src/orders.ts` is the app's code, `test/orders.test.ts` its consumer test, written with the TypeScript SDK and its Vitest integration. Its `package.json` does not name the SDK, because `run.sh` installs the one packed from this tree. A real project installs a release's tarball: `npm install --save-dev https://github.com/rholshausen/pact-janus/releases/download/typescript-sdk/v<version>/pact-janus-<version>.tgz` |
| `steps/` | The two changes steps 4 and 5 make, as patches |
| `run.sh` | The walkthrough. It trims Vitest's stack traces and prints the SDK's own failure report; everything else on screen is the commands' own output |
| `demo.tape` | The VHS script that records `run.sh --pace 5` |

The provider side is the repo's sample provider, unchanged: `samples/order-service`, its
`verifier.janus.yaml` and hooks, and the shape its own tests recorded (`shapes/`, plan task 7.2).

## Choices worth knowing about

- **`shippedAt` varies on its own, less one exclusion.** The sample provider cannot produce a
  `SHIPPED` order without a shipping date, and it says so (`state-unreachable`). The consumer's
  code shows a shipping date whatever the status, so it relies on no correlation between the two,
  and the combination is excluded with its reason (`exclude`, ADR 0080) rather than written as a
  `oneOf` on the status (ADR 0081, which is for a correlation the consumer's code relies on). The
  contract still admits a `SHIPPED` order with no date; the exclusion is only what the test does
  not exercise, and `janus check` repeats its reason wherever that leaves a field untested. The
  provider state binds `shipped` to `shippedAt`'s presence (`whenVariant("shippedAt", "present")`),
  so a `PENDING` order is served with and without a date, as the provider can send it.
- **The first `check` warns rather than blocks.** That is the default (ADR 0016). `--on-finding block`
  would make step 3 the failure instead.
