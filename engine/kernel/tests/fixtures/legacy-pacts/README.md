# Legacy pact fixtures

Real-world v1–v4 pacts, most copied from a local `pact-jvm` checkout: its consumer-test build output
(`consumer/junit5/build/pacts/`, `consumer/junit/build/pacts/`) and its provider test resources
(`provider/src/test/resources/pacts/`). They exercise the engine against artifacts actual SDKs
produced rather than only the synthetic cases this repo authors itself. pact-jvm is Apache-2.0,
license-compatible with this project. The two `v4-plugin*` files are trimmed from `pact-plugins`' examples
(MIT): their interactions as the examples' consumers wrote them, with the protobuf descriptor and most rules cut.

These supplement, and never replace, the official `pact-specification` spec test-case corpus
(reuse-inventory.md: "data" verdict) that task 3.5 validates the legacy matching-rule compiler
against.

| File | Spec version | Written by | Notes |
|---|---|---|---|
| `consumer-provider-v4.json` | v4 | pact-jvm 4.7.6 | plain HTTP, two interactions, no matching rules |
| `V3Consumer-ProviderStateService.json` | v3 | pact-jvm 4.7.6 | provider states, `type` and `regex` matching rules |
| `eachkeylike__consumer-eachkeylike_provider.json` | v4 | pact-jvm 4.7.6 | `eachKeyLike` matching rule |
| `test_consumer_v3-MessageProvider.json` | v3 | pact-jvm 4.7.6 | message pact (`messages`, not `interactions`) |
| `XMLConsumer-XMLProvider.json` | v3 | pact-jvm 4.7.6 | XML bodies, with rules on attributes and `#text` (ADR 0039) |
| `zoo_app-animal_service.json` | v1-era | pact_gem 1.0.9 | five interactions, a bare `provider_state` string, lower-case `get` |
| `v4-plugins-mixed.json` | v4 | trimmed from pact-plugins' examples | a plain HTTP interaction beside a CSV one over HTTP, a gRPC one and an Avro message, each naming its plugin in `pluginConfiguration` (staged plan S1.8) |
| `v4-plugin-matcher-only.json` | v4 | trimmed from pact-plugins' `creditcard` example | a plugin that contributes only a matcher and a generator, named in the metadata alone, beside an interaction that uses neither (S1.8) |

The pact_gem file is there for exactly the reason it looks out of place: it is a Ruby SDK's
decade-old output, with none of v3's structure, and plan task 5.4's claim is that a provider
verifies pacts like it without either side changing. `tests/legacy_verification.rs` runs it.
