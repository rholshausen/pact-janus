---
default: major
---

# An interaction that needs a pact plugin is refused by name

- **Breaking: a v1–v4 interaction that needs a pact-plugins plugin fails** as `component-unavailable`
  before anything runs for it: no state is set up and nothing is sent. `details.component` is
  `pact-plugin/<name>`, `details.plugins` lists every plugin it needs, and `details.reason` is
  `pact-plugin` (ADR 0045). Until this release, a CSV or other plugin-backed HTTP interaction was
  verified with its body compared as text, which could pass while checking only part of the pact.
  A gRPC, protobuf or Avro message interaction was skipped and is now reported.
- **An interaction needs a plugin** when its `pluginConfiguration` names one, or when it uses a
  matcher only a plugin provides, which needs every plugin the pact's metadata names.
- **The rest of the pact is verified**, and the summary counts each refusal as a failure.
- **`upgrade/pact` drops such an interaction** with an `interaction-dropped` finding naming the
  plugin, and **`verification/explain` refuses it** with the same error.
