---
engine: major
---

# A provider shape is recorded by the operation each test names, and written by the engine's host

Staged plan S4.1, ADR 0086.

- **`observe` names an operation.** `provider-shape-session/observe` takes `operation`, shapes over the
  initiating part's slots, which become the recorded entry's `selector`, so a provider's own tests
  record a shape without knowing a consumer's words. With the initiating part in `parts`, an
  operation that does not admit it is `interaction-invalid` at `/operation`.
- **`observe` takes an arrival.** `parts` are the transport's own, a content slot's octets tagged
  `base64` and labelled with their type, and the engine decodes them through its content components.
  Only the answering part is recorded. Octets a component cannot decode are recorded as octets, with
  a `warnings` entry in the result. A plain JSON value, as before, is still read as itself.
- **`create` takes `kind`**, `http` unless given, and reads what that transport declares. An engine
  with no transport of that kind answers `component-unavailable`: an embedding that records shapes
  must register its transports, as one that serves or drives does.
- **Outcomes are kept apart.** A transport declares `outcome-slots` (component-interfaces spec §5.10),
  HTTP's the response's `status`, which an observation that names an operation writes into its
  selector. A 200 and a 404 of one operation are two entries.
- **Credentials are withheld.** A member the transport declares a credential, such as `set-cookie`,
  is recorded as `any`, never with its value.
- **`finalise` writes.** `write: { path, evidence, source }`, behind `contract-writing`, has the
  engine's host write the shape and keep each source's counts in `evidence`, so several test files,
  in sequence or in parallel workers, write the shape one session recording them all would have. The
  result has `written` beside `provider-shape`.
- **`http:path`.** The built-in HTTP component runs a new operator, `http:path`, whose member
  `template` is a path template: each `{name}` matches one or more characters that are not `/`. An
  embedding that registers the HTTP component's matcher actions (`ACTIONS`) gets it with no change.
- **Selecting.** `subsumption/check` runs a selector with the engine's matchers, so it may use
  `http:path`. When an entry naming an interaction's states and one naming none both select it, the
  one naming them is used, where the two were a tie.
