---
default: patch
---

# Documents that are not what they claim are refused, and `janus` says which file

The first fixes of staged plan S1.29's review of the CLI's output.

- **`upgrade/pact` refuses a document that is not a pact.** A document without `consumer` and
  `provider` objects and an `interactions` or `messages` array used to become an empty contract, reported
  as an exact conversion. It is now `contract-invalid`, with a problem at the empty pointer. The same
  check now covers `verification/explain`'s v1–v4 path.
- **`verification/explain` refuses a variant the interaction does not record**, as `variant-not-found`
  with the `recorded` ids in its details, as a run does (ADR 0041). For a v1–v4 interaction, which
  records only `base`, any other id used to be ignored. For a contract it used to be `malformed-frame`.
- **Messages.** A source document that is not readable is `contract-invalid` with "the document is not a
  contract or pact this engine can read". It used to say "the session's interactions cannot produce a
  valid contract", which is `consumer-session/finalise`'s message. A source that is neither a contract nor a pact
  is now "neither a Janus contract nor a v1–v4 pact". The codes and details are unchanged.

`janus` follows:
- An error about one of `verify`'s files names that file, and its positions are within it.
- `--provider-url`, `--broker-url`, `--url`, `--webhook-callback-url`, `--state-change-url` and
  `publish --broker-base-url` must be URLs with a host, checked as the command line is read.
- `explain --index` past the last interaction is a usage error, and `--spec` is suggested only for a
  document the engine could not identify.
- `publish` no longer says a broker it never reached "did not accept" the contracts.
- `pact_models`' warnings about a pact with no metadata are no longer logged by default.
- `--log-level trace` no longer makes loading a WASM component take seconds: the compiler's own
  `trace` is held back, as the HTTP libraries' is.
- The help now says `trace`, not `debug`, shows the protocol frames.
