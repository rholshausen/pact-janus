# Legacy pact fixtures

Real-world v1–v4 pacts, copied from a local `pact-jvm` checkout's consumer-test build output
(`consumer/junit5/build/pacts/`, `consumer/junit/build/pacts/`), used to exercise
`pact_models::pact::load_pact_from_json` against artifacts an actual SDK produced rather than only
the synthetic cases this repo authors itself. pact-jvm is Apache-2.0, license-compatible with this
project.

These supplement, and never replace, the official `pact-specification` spec test-case corpus
(reuse-inventory.md: "data" verdict) that task 3.5 validates the legacy matching-rule compiler
against.

| File | Spec version | Notes |
|---|---|---|
| `consumer-provider-v4.json` | v4 | plain HTTP, two interactions, no matching rules |
| `V3Consumer-ProviderStateService.json` | v3 | provider states |
| `eachkeylike__consumer-eachkeylike_provider.json` | v4 | `eachKeyLike` matching rule |
| `test_consumer_v3-MessageProvider.json` | v3 | message pact (`messages`, not `interactions`) |
| `XMLConsumer-XMLProvider.json` | v3 | XML body |
