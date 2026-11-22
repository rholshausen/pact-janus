# Writing a Janus component

This guide is for someone writing a **component**: a WebAssembly module the Janus engine loads to read a
content type it does not know, or to check a value in a way its own operators cannot. It takes you from
an empty directory to a component that is published, signed, declared by a project, and recorded in the
contracts and verification results it took part in.

It is a guide, not the rules. Every rule is in the
[component interfaces specification](specs/component-interfaces/spec.md) ("the spec" below), and this guide
links the section that holds it. Where the two disagree, the spec is right and this guide has a bug.

What you may depend on is set by
[ADR 0104](decisions/0104-the-component-interface-is-generally-available-for-content-and-matcher-components-over-wasm.md):
**content and matcher components, loaded as WebAssembly**, through component protocol v1. §11 lists what is
specified but not yet callable.

## Contents

1. [What a component is](#1-what-a-component-is)
2. [What you need](#2-what-you-need)
3. [The pipe: one function, JSON frames](#3-the-pipe-one-function-json-frames)
4. [The handshake](#4-the-handshake)
5. [A content component](#5-a-content-component)
6. [A matcher component](#6-a-matcher-component)
7. [Errors](#7-errors)
8. [Building](#8-building)
9. [Testing](#9-testing)
10. [Publishing, signing and declaring](#10-publishing-signing-and-declaring)
11. [What is not available yet](#11-what-is-not-available-yet)
12. [What stays stable](#12-what-stays-stable)
13. [Checklist](#13-checklist)

## 1. What a component is

The engine matches **documents**, never octets. A content type it has a component for (JSON, XML, text,
`application/x-www-form-urlencoded`, `multipart/*`) is decoded into a document before any shape is
applied, and encoded from one when a mock serves a reply or a verifier sends a request. A body of any
other type is just bytes, so nothing inside it can be checked. A content component is how a project
teaches the engine a type: `text/csv`, protobuf, YAML.

The document model is JSON values plus byte strings
([shape language spec §2.1](specs/shape-language/spec.md)). A shape, such as "an array of at least one
object whose `id` is a string", is written against that document, and works the same whatever octets the
document came from.

A matcher component contributes **operators**, which shapes can name (`csv:column`), and **actions**,
which run them. A content component often needs a matcher too, because its type changes what a core
operator means: CSV has no numbers, so `integer` of a CSV field has to mean "text that spells an integer"
(§5.6).

One component can be both. Its **name** is its namespace: everything it contributes is called
`<name>:<something>` ([spec §2.4](specs/component-interfaces/spec.md#24-the-namespace-rule)).

A component never calls the engine. The engine calls it, one request at a time, and it answers each one
([spec §3.1](specs/component-interfaces/spec.md#31-frames)).

## 2. What you need

- A language that builds a **WASI 0.2 component** (`wasm32-wasip2`) exporting the world in
  [`wit/component.wit`](specs/component-interfaces/wit/component.wit). This guide uses Rust: `rustup
  target add wasm32-wasip2` and the [`wit-bindgen`](https://crates.io/crates/wit-bindgen) crate. Any
  toolchain that can export that world works.
- A copy of `component.wit` in your project. It is 19 lines, and it never changes
  ([spec §9.2](specs/component-interfaces/spec.md#92-the-wasm-binding-out-of-tree-default)).
- The `janus` CLI from a [release](https://github.com/pact-foundation/pact-janus/releases)
  (`janus-<target triple>`), to run your component in the engine and to publish it.
- For an end-to-end consumer test, a Janus SDK: the TypeScript one
  (`npm install pact-janus`) or the JVM one. Optional: `janus verify` alone exercises a component.

## 3. The pipe: one function, JSON frames

The world exports one function:

```wit
call: func(request: list<u8>) -> list<u8>;
```

Each call carries one **request frame**, UTF-8 JSON with no length header, and returns exactly one
**response frame**. The frames are the engine protocol's
([`frame.schema.json`](specs/engine-protocol/schemas/v1/frame.schema.json)):

```json
{ "type": "request", "id": "7", "op": "content/decode", "body": { "content-type": "text/csv", "value": { … } } }
```

```json
{ "type": "response", "id": "7", "ok": { "document": [ … ] } }
```

```json
{ "type": "response", "id": "7", "error": { "code": "decode-failed", "category": "document", "message": "…" } }
```

- Echo the request's `id`. A response carries `ok` or `error`, never both. On this pipe `error` is a
  `ComponentError` (§7), not the engine-error the frame schema names for the host's pipe.
- `body` and `ok` are the operation's documents, defined by the schemas under
  [`schemas/v1/`](specs/component-interfaces/schemas/v1/). Ignore members you do not know, and never
  depend on the engine reading a member the schema does not define
  ([engine-protocol spec §2.2](specs/engine-protocol/spec.md)).
- An `op` you do not implement, on an interface you declared, is answered with the error
  `operation-unsupported`, its `details.op` naming it.
- A frame you cannot parse is answered with `malformed-frame` and an empty `id`. A frame that parses but
  is not a request, or whose `body` lacks a member the operation's schema requires, is `malformed-frame`
  too, with the request's `id` when it has one ([engine-protocol spec §4.4](specs/engine-protocol/spec.md#44-malformed-frames)).
- `component/shutdown`, with body `{}`, may arrive before an instance is dropped. Answer `{}`. Never
  depend on it: an instance can vanish without one.
- The first request on every instance is `component/hello` (§4). Answer anything earlier with
  `handshake-required`.

Rules that shape how you write it ([spec §3.4](specs/component-interfaces/spec.md#34-instances-lifetime-and-deadlines)):

- **Keep no state but one bit.** The engine may use a fresh instance for every call, with its own
  handshake, so nothing you remember between calls may change an answer. The one thing an instance
  remembers is whether it has been greeted, so that it can answer `handshake-required` (§8 shows how).
- **Calls are serial.** You are never called concurrently.
- **Every call has a deadline** (10 seconds unless a project's declaration says otherwise). A call that
  runs past it is stopped and reported as `component-timeout`.
- **Never panic or trap by design.** A trap is reported for you as `component-trapped`, the instance is
  thrown away, and the run fails. Answer an error instead (§7).
- **You get nothing ambient.** No environment variables, no files, no sockets, unless the project grants
  them ([spec §10.4](specs/component-interfaces/spec.md#104-grants)). The imports Rust's `std` brings are
  linked to nothing, which is harmless. Importing `wasi:sockets` or `wasi:http` without a `network` grant
  fails the load.

## 4. The handshake

The engine says who it is and what it gave you
([spec §3.2](specs/component-interfaces/spec.md#32-the-handshake),
[`handshake.schema.json`](specs/component-interfaces/schemas/v1/handshake.schema.json)):

```json
{ "component-protocol-versions": [1],
  "engine": { "name": "janus-engine", "version": "0.8.0" },
  "grants": { "env": [], "fs": [], "network": false },
  "capabilities": { "embedded-content": {} } }
```

You answer with who you are and what you contribute:

```json
{ "component-protocol-version": 1,
  "component": { "name": "yaml", "version": "1.0.0" },
  "interfaces": ["content"],
  "contributes": {
    "content-types": [
      { "media-type": "application/yaml",
        "degradations": [ { "code": "tags-dropped", "message": "YAML tags are not part of the document." } ] } ] },
  "capabilities": {} }
```

- Pick `1` from `component-protocol-versions`. If it is not there, answer `protocol-version-unsupported`.
- `name` is `[a-z][a-z0-9-]*`, and it must be the name a project declares you under, or the load fails.
  `flow`, `value`, `expect`, `check`, `match` and `legacy` are reserved.
- `version` is a semantic version. A contract that needs you pins your **major** (§10.4), so a major is a
  promise about what your operators and documents mean
  ([spec §12.2](specs/component-interfaces/spec.md#122-what-a-component-may-change)).
- `interfaces` is what you implement: `content`, `matcher`, or both.
- Every name under `contributes` is in your namespace: `yaml:anchor`, never `anchor` or `json:anchor`.
- **The handshake is the only description of you that counts.** There is no manifest. What it declares,
  the engine must be able to call: declaring something §11 lists as not callable fails the load.

The engine checks all of this when it loads you, and refuses the load as `component-invalid` with a
message saying which rule you broke. That is the fastest way to find out (§9.3).

## 5. A content component

The content operations are in [spec §6](specs/component-interfaces/spec.md#6-the-content-interface) and
[`content.schema.json`](specs/component-interfaces/schemas/v1/content.schema.json).

| Operation | Required | What it does |
|---|---|---|
| `content/decode` | yes | octets to a document |
| `content/encode` | yes | a document to octets |
| `content/compile` | yes | says whether you can carry a shape at all |
| `content/detect` | no | says whether octets look like your type |
| `content/label` | no | says a member of your document has a type of its own |

You declare the types you handle in `contributes.content-types`, each a media type or a pattern
(`text/csv`, `application/*+yaml`). Parameters are ignored when the engine routes, so `text/csv` handles
`text/csv; charset=utf-8`. Your component is asked for a slot when a project declares you and an
interaction says the slot is of a type you handle
([contract-file spec §5.5](specs/contract-file/spec.md)). A declared component is asked before the
engine's built-in ones, so you can replace how the engine reads JSON if you need to.

### 5.1 Slot values: how octets arrive

A **slot value** ([`parts.schema.json`](specs/component-interfaces/schemas/v1/parts.schema.json)) wraps
content with a tag saying how to read it:

```json
{ "content": "aWQsc3RhdHVzCjY2LFBFTkRJTkcK", "encoded": "base64", "content-type": "text/csv" }
```

| `encoded` | `content` is |
|---|---|
| `base64` | octets, base64 encoded. What a transport hands you. |
| `text` | the octets as a UTF-8 string |
| absent or `json` | a JSON value. A document a contract recorded, already decoded. |

Accept all three. For `json`, the value is already your document: return it as it is. `content-type`, when
present, is how the octets were labelled where they arrived, which can differ from the type you are asked
to decode as.

### 5.2 Decode

```json
{ "content-type": "text/csv", "value": { "content": "aWQsc3RhdHVzCjY2LFBFTkRJTkcK", "encoded": "base64" } }
```

```json
{ "document": [ { "id": "66", "status": "PENDING" } ] }
```

- The document is JSON. Make it the structure a consumer would write a shape against, keyed by the names
  your format gives things.
- A document that is a byte string goes back as `{ "document": "<base64>", "encoded": "base64" }`.
- **Objects keep their member order**, and the engine keeps it too
  ([engine-protocol spec §2.4](specs/engine-protocol/spec.md)). Write members in the order the octets
  have them. With `serde_json`, turn on its `preserve_order` feature.
- **Octets that are not your type are an error, never a guess**: `decode-failed`, with a message a person
  can act on, and the line or path in `details` when you have one. The engine reports it at the slot, in
  your words ([spec §6.1](specs/component-interfaces/spec.md#61-decoding-is-the-bytes-boundary)).
- **Empty octets are asked of you too.** If your format has an empty document (zero NDJSON records, an
  empty YAML stream), decode to it: an empty slot is then that document. If it has none, refuse, and the
  engine treats the empty slot as no body, which is not a failure. A slot value with no `content` member
  is an empty slot.
- **Keep what the shape language distinguishes.** `integer` and `decimal` are told apart by how a number
  is written, so `100` is an integer and `100.0` a decimal
  ([shape language spec §4.2](specs/shape-language/spec.md#42-value-operators)). A format that writes
  numbers as JSON does should keep the difference, and integers too large for 64 bits.
- A request whose `content-type` is not one you declared is `unsupported-content-type`. Compare its type
  and subtype, ignoring parameters and case.

### 5.3 Degradations: say what your format loses

If your format cannot express a distinction a shape can make, such as CSV with no numbers, or a format
with no null, say so ([spec §6.4](specs/component-interfaces/spec.md#64-declaring-degradations)):

- **Statically**, in the handshake's `degradations` for the content type, each with a `code` you choose
  and a `message`.
- **Per decode**, in the result's `degradations`, each with a `path` into your document where the loss
  happened, written `$`, `$.name`, `$[0]`, `$[0].name`:

```json
{ "document": [ { "total": "12.50" } ],
  "degradations": [ { "code": "numeric-lexical", "path": "$[0].total", "message": "CSV has no numeric type" } ] }
```

A verification shows each per-decode degradation as a warning, located in the reply.

A degradation is a distinction lost *inside* a document you do decode. A shape no document of your type
could ever match, such as an NDJSON body that is one object, is not a degradation: refuse it in
`content/compile` (§5.5).

### 5.4 Encode

The inverse of decode, used when a mock serves your type or a verifier sends it
([spec §6.2](specs/component-interfaces/spec.md#62-encoding-is-how-variants-become-traffic)):

```json
{ "content-type": "text/csv", "document": [ { "id": "66", "status": "PENDING" } ],
  "shape": { "shape": "each-like", "min": 1, "items": { "shape": "object", "members": { … } } } }
```

```json
{ "value": { "content": "aWQsc3RhdHVzDQo2NixQRU5ESU5HDQo=", "encoded": "base64", "content-type": "text/csv" } }
```

- **`decode(encode(d))` must equal `d`** for every document your decode produces. A contract records an
  example your encode wrote, and your decode will read it back.
- Members arrive in the order the shape declares them. Write them in that order.
- `shape`, when present, is the shape the document was generated from, in the
  [shape language](specs/shape-language/spec.md)'s JSON form. Use it only for what the document cannot
  say. An empty CSV document names no columns, so the CSV component writes its header row from the
  shape's members. It is absent when no shape produced the document, so encode must work without it.
- The result's `content-type` is the type the octets are labelled with. Echo the requested one, or add to
  it what the octets need, such as a boundary.
- A document you cannot write as your type is `encode-failed`.

### 5.5 Compile, detect and label

- **`content/compile`** gives you a slot's `shape` and its `path` when an interaction arrives. Answer `{}`
  if you can carry it. Answer an error if no document of your type could ever match it: the built-in text
  component answers `encode-failed` for a shape that admits no string. The engine refuses the interaction,
  reporting your error under `compile-failed`. The shape is in the shape language's JSON form, its
  operators listed in [shape language spec §4.1](specs/shape-language/spec.md#41-overview):
  answer `{}` for an operator you do not know, which a component's or a later version's may be. You return nothing else. A component writes no
  plan ([spec §6.3](specs/component-interfaces/spec.md#63-saying-what-a-value-operator-means)), and
  `fragment`, which old documents mention, is ignored.
- **`content/detect`** answers whether octets with no type look like yours, for the `content-type`
  operator: `{ "media-type": "text/csv", "confidence": 0.6 }`, or `{}`. If you cannot tell, answer
  `operation-unsupported` ([spec §6.5](specs/component-interfaces/spec.md#65-detection)). The engine asks
  each component in turn and takes the first that names a type, so claim only octets that are clearly
  yours. Today it does not weigh `confidence` and sends no `hint`.
- **`content/label`** is for a document whose members carry a type of their own, as a multipart part's
  body does. Most components answer `operation-unsupported`.

### 5.6 Substitutions: what a core operator means for your type

If your documents hold strings where a consumer means numbers, the engine's own `integer` check fails
them. Say what each affected operator means for your type, as one of your actions, under the content
type's `substitutions` ([spec §6.3](specs/component-interfaces/spec.md#63-saying-what-a-value-operator-means)):

```json
{ "media-type": "text/csv",
  "substitutions": { "integer": "csv:integer", "number": "csv:number", "equality": "csv:equality" } }
```

The engine still compiles the shape itself. Wherever it would check `integer` in a slot you decode, it
runs your `csv:integer` instead, through `matcher/apply` (§6). So you need the `matcher` interface, and
each action must be in `contributes.actions`. Only value operators can be substituted: `any`, `equality`,
`type`, `string`, `number`, `integer`, `decimal`, `boolean`, `null`, `regex`, `datetime`, `date`, `time`,
`include`, `semver`, `not-empty` and `any-of`.

### 5.7 Content inside your content

If your format holds content of another type, such as an envelope around a JSON payload, do not decode
the payload yourself. Ask the engine, and it decodes the payload with whatever component the project has
for that type ([spec §6.6](specs/component-interfaces/spec.md#66-embedded-content)). This needs the
`embedded-content` capability in the engine's handshake. Answer the first `decode` with what you need, and
`null` for the document:

```json
{ "document": null,
  "embedded": [ { "id": "payload", "content-type": "application/json",
                  "value": { "content": "eyJpZCI6IDF9", "encoded": "base64" } } ] }
```

The engine repeats the request with the answers under `embedded`, by id: each a decode result, or
`{ "error": … }`. Answer that request with your document. Encode works the same way, with a `document`
and an `at` pointer to where it sits in yours. Most components never need this.

## 6. A matcher component

The matcher operations are in [spec §7](specs/component-interfaces/spec.md#7-the-matcher-interface) and
[`matcher.schema.json`](specs/component-interfaces/schemas/v1/matcher.schema.json).

| Operation | Required | What it does |
|---|---|---|
| `matcher/apply` | yes | runs one of your actions on values |
| `matcher/compare` | for an `exact` operator | decides whether one shape admits everything another does |
| `matcher/prefix` | no | the text every value an operator admits begins with |

### 6.1 Actions and operators

An **action** is something the engine's plan can run: `csv:integer`. An **operator** is something a shape
can name: `{ "shape": "csv:column", "type": "integer" }`. A shape's component operator compiles to the
action of the same name, so contribute an action for each operator
([spec §7.1](specs/component-interfaces/spec.md#71-a-component-operator-compiles-to-its-own-action)):

```json
"contributes": {
  "actions": [ { "name": "csv:integer" }, { "name": "csv:column" } ],
  "operators": [ { "name": "csv:column", "comparability": "exact" } ] }
```

### 6.2 Apply

```json
{ "action": "csv:column", "config": { "arguments": [ { "type": "integer" } ] },
  "values": [ { "content": "42", "path": "$.response.body[0].items" } ] }
```

```json
{ "results": [ { "status": "ok" } ] }
```

- `values[i].content` is the value under test, tagged like a slot value (`encoded: base64` for bytes).
  `path` says where it came from.
- `config.arguments` are the action's further inputs. For an operator's action, it is one object: the
  operator's members from the shape (here `{ "type": "integer" }`). For a substituted action, it is the
  core operator's: `equality`'s example, `regex`'s pattern, `any-of`'s literals, nothing for `integer`
  ([spec §7.2](specs/component-interfaces/spec.md#72-batching)).
- Return one result per value, in order. A result is `{ "status": "ok" }`, or `{ "status": "error",
  "message": "expected text that spells an integer, got \"many\"" }` for a mismatch. A mismatch is a
  result, **not** a `ComponentError`: errors are for "I could not do my job" (§7).
- You get one value per call unless you declare the `batch-apply` capability in your handshake's
  `capabilities`.

### 6.3 Comparability

`janus check` decides whether a provider can break a consumer without running either. For your operator,
the check only knows what you tell it ([spec §7.5](specs/component-interfaces/spec.md#75-declaring-comparability)):

| `comparability` | Means | You must |
|---|---|---|
| `exact` | the check can decide by asking you | implement `matcher/compare`, and contribute an action of the operator's name |
| `conservative` | the check decides identity and obvious widening | contribute an action of the operator's name |
| `opaque`, or absent | the check decides only identical shapes | nothing |

```json
{ "operator": "csv:column", "provider": { "shape": "csv:column", "type": "integer" }, "consumer": { "shape": "string" } }
```

```json
{ "verdict": "yes", "reason": "every integer column is a string" }
```

`verdict` is `yes`, `no` or `unknown`. **Answer `unknown` rather than guess.** A wrong `yes` tells a team
it is safe to deploy something that breaks. `prefix` is optional and only speeds the check up
([spec §7.6](specs/component-interfaces/spec.md#76-declaring-a-prefix)).

## 7. Errors

An error is a `ComponentError` ([spec §11](specs/component-interfaces/spec.md#11-errors),
[`component-error.schema.json`](specs/component-interfaces/schemas/v1/component-error.schema.json)):

```json
{ "code": "decode-failed", "category": "document", "message": "line 3: expected ',' or end of row",
  "details": { "content-type": "text/csv", "line": 3 } }
```

| Category | Codes you will use |
|---|---|
| `protocol` | `malformed-frame`, `handshake-required`, `operation-unsupported`, `protocol-version-unsupported` |
| `document` | `unsupported-content-type`, `decode-failed`, `encode-failed`, `unknown-action`, `invalid-config` |
| `component` | `capability-denied`, `unavailable` |

The engine passes your error through as you wrote it, so write the message for the person reading a
failed test. Never set `source`: `"engine"` there marks an error the engine made on your behalf, such as a
timeout.

## 8. Building

`Cargo.toml`:

```toml
[package]
name = "janus-yaml"
version = "1.0.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]   # cdylib for the component, rlib so native tests can call your code

[dependencies]
serde_json = { version = "1", features = ["preserve_order"] }
base64 = "0.22"   # slot values carry octets as base64

[target.'cfg(target_arch = "wasm32")'.dependencies]
wit-bindgen = "0.63"

[profile.release]
opt-level = "s"
lto = true
strip = true
```

`src/lib.rs`, with your logic in a plain type so it can be tested natively. An instance holds the one
thing it may remember (§3):

```rust
/// One live copy of the component. It remembers only whether it has been greeted.
#[derive(Default)]
pub struct Instance {
  greeted: bool,
}

impl Instance {
  /// One request frame in, one response frame out.
  pub fn call(&mut self, request: &[u8]) -> Vec<u8> {
    // parse the frame; answer `handshake-required` until `component/hello` has set `greeted`;
    // dispatch on "op"; build the response frame
    todo!()
  }
}

#[cfg(target_arch = "wasm32")]
mod bindings {
  use std::cell::RefCell;

  wit_bindgen::generate!({ world: "component", path: "wit" });

  thread_local! {
    // A WebAssembly instance is single-threaded, and is this component's instance.
    static INSTANCE: RefCell<super::Instance> = RefCell::default();
  }

  struct Component;

  impl exports::pact::janus_component::pipe::Guest for Component {
    fn call(request: Vec<u8>) -> Vec<u8> {
      INSTANCE.with(|instance| instance.borrow_mut().call(&request))
    }
  }

  export!(Component);
}
```

Native tests make an `Instance::default()` of their own, so they never share one.

Put `component.wit` at `wit/component.wit`, then:

```bash
cargo build --release --target wasm32-wasip2
# target/wasm32-wasip2/release/janus_yaml.wasm
```

`third-party/janus-csv` in the Janus repository is a complete component built this way, if you want to
see one.

## 9. Testing

### 9.1 Natively, frame by frame

Because `call` is a plain function, `cargo test` can drive it with frames and check what comes back:
the handshake, a decode of a real body, a decode of a broken one, `decode(encode(d)) == d`, an `op` you do
not implement. Most of your tests belong here.

### 9.2 Against conformance cases

A conformance case ([spec §9.4](specs/component-interfaces/spec.md#94-dual-binding-conformance),
[`conformance-case.schema.json`](specs/component-interfaces/schemas/v1/conformance-case.schema.json)) is a
JSON file of calls on your component, each with the result or error it must answer:

```json
{ "name": "ndjson/decode", "component": "ndjson", "interface": "content",
  "calls": [
    { "op": "content/decode",
      "body": { "content-type": "application/x-ndjson",
                "value": { "content": "{\"id\":1}\n", "encoded": "text" } },
      "expect": { "ok": { "document": [ { "id": 1 } ] } } },
    { "op": "content/decode",
      "body": { "content-type": "application/x-ndjson", "value": { "content": "{", "encoded": "text" } },
      "expect": { "error": { "code": "decode-failed" } } } ] }
```

An `ok` constrains only the members it names, so a member you add later does not break a case. An
`error` is matched on its `code` and on whatever else it names, never its `message`. Keep the cases beside
your component and run them against the built `.wasm`, which loads it as a run does, with no grants, and
calls it through the same binding:

```sh
janus component check target/wasm32-wasip2/release/janus_ndjson.wasm conformance/
```

It exits 0 when every case passes and 1 when one fails, naming the call and where your answer differs.
`component/hello` is answered with the handshake the load made, so a case can assert what you contribute.
The CSV component's are in `third-party/janus-csv/conformance/`, and the built-in components' in
`corpora/components/`, which show what each operation answers.

### 9.3 In the engine

Declare the built `.wasm` in a project configuration, `verifier.janus.yaml`:

```yaml
version: 1
components:
  - name: ndjson
    source: { kind: file, reference: target/wasm32-wasip2/release/janus_ndjson.wasm }
```

A `file` reference is relative to the configuration file's directory, or absolute.

To verify a provider you need a **contract**: what a consumer test writes, an interaction with the
variants it was exercised with. An SDK writes one for you (below). Without one, write it by hand. This is
the smallest useful one, as the engine writes it
([contract-file spec](specs/contract-file/spec.md) has every member):

```json
{ "$format": "janus-contract/1",
  "consumer": { "name": "reporting" },
  "provider": { "name": "order-service" },
  "interactions": [ {
    "description": "every order, as NDJSON",
    "transport": { "kind": "http", "mode": "passive" },
    "requires": [ { "component": "content/ndjson", "min-version": 1 } ],
    "content-types": { "response": { "body": "application/x-ndjson" } },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders.ndjson" } },
      "response": {
        "status": { "shape": "equality", "example": 200 },
        "body": { "shape": "each-like", "min": 1, "max": 1,
                  "items": { "shape": "object", "members": {
                    "id": { "shape": "integer", "example": 66 },
                    "status": { "shape": "string", "example": "PENDING" } } } } } },
    "selection": {
      "variants": [ {
        "id": "base", "origin": "base", "assignment": [],
        "parts": {
          "request": { "method": { "content": "GET" }, "path": { "content": "/orders.ndjson" } },
          "response": { "status": { "content": 200 },
                        "body": { "content": [ { "id": 66, "status": "PENDING" } ],
                                  "content-type": "application/x-ndjson" } } } } ],
      "report": { "algorithm": "janus-ipog-v1", "boundaries": true,
                  "budgets": { "exhaustive-threshold": 8, "max-variants": 50 },
                  "coverage": { "covered": 0, "dropped": 0, "removed": 0, "targets": 0 },
                  "selected": 1, "space": { "dimensions": 0, "exact": true, "size": 1 },
                  "strategy": "exhaustive" } } } ] }
```

- `content-types` says the slot is your type, so the engine decodes it with you. `requires` names you, so
  a run that has not declared you fails before it starts instead of failing every interaction.
- Each variant is verified as it was exercised: pinned to its point. `min: 1, max: 1` has one variant.
  A range such as `min: 1` has more (one item, and more than one), and the provider must produce each,
  which is what provider states are for
  ([lifecycle-hooks spec §8.4](specs/lifecycle-hooks/spec.md#84-http); `janus verify -s <url>` sends each state
  change to an endpoint, in the body format that section gives).

Then:

- `janus verify contract.json --provider-url http://localhost:8080 --config verifier.janus.yaml`
  verifies the contract against a running provider, decoding its replies with your component. With
  `--output json`, the summary's `components` names your component and the digest of the file it loaded:
  the way to be sure it was used.
- `janus explain contract.json --config verifier.janus.yaml --plan` loads your component and prints the
  plan an interaction compiles to. A handshake that breaks a rule fails here with `component-invalid`
  and says which, and a shape your `compile` refuses fails with your message. The plan names your
  component only where it substitutes an operator (§5.6). `janus explain interaction.json --spec`
  reads an interaction alone, with no `selection`.

From a consumer test, the TypeScript SDK declares a slot's type with `content(...)` and the component with
the `components` option:

```ts
const janus = new Janus({ consumer: "reports", provider: "orders", components: [
  { name: "yaml", source: { kind: "file", reference: "target/wasm32-wasip2/release/janus_yaml.wasm" } } ] });

janus.interaction("every order, as YAML")
  .request({ method: "GET", path: "/orders.yaml" })
  .response({ status: 200, body: content("application/yaml", eachLike({ id: string("66") })) });
```

The mock then serves bodies your `encode` wrote, and the contract it writes names your component.

## 10. Publishing, signing and declaring

### 10.1 Publish to an OCI registry

Either command publishes the `.wasm` as an artifact the engine loads
([spec §10.3](specs/component-interfaces/spec.md#103-oci-artifacts-and-integrity)):

```bash
janus component push target/wasm32-wasip2/release/janus_yaml.wasm ghcr.io/acme/janus-yaml:1.0.0
# or, with the Bytecode Alliance's wkg:
wkg oci push ghcr.io/acme/janus-yaml:1.0.0 target/wasm32-wasip2/release/janus_yaml.wasm
```

`janus component push` writes the artifact's config from your handshake and prints the manifest's
**digest**. Credentials come from `JANUS_OCI_USERNAME` and `JANUS_OCI_PASSWORD`. `wkg` writes a timestamp
into its config, so every push of the same bytes is a new digest. Either way, the digest is what a project
pins.

`janus component pull ghcr.io/acme/janus-yaml:1.0.0` fetches it, checks it, and prints the declaration a
project would write, digest included.

### 10.2 Sign it

Sign the pushed manifest with [Sigstore's `cosign`](https://docs.sigstore.dev/cosign/signing/signing_with_containers/),
which attaches the signature to the artifact as an OCI referrer. In GitHub Actions, signing is keyless,
with the workflow's own identity:

```bash
cosign sign --yes ghcr.io/acme/janus-yaml@sha256:<digest>
```

### 10.3 What a project declares

```yaml
components:
  - name: yaml
    source:
      kind: oci
      reference: ghcr.io/acme/janus-yaml:1.0.0
      digest: sha256:<digest>
      signature:
        identity: https://github.com/acme/janus-yaml/.github/workflows/release.yml@refs/tags/v1.0.0
        issuer: https://token.actions.githubusercontent.com
    grants: { network: false }
    limits: { deadline-ms: 10000 }
```

With a `digest`, the engine fetches those bytes and no others, whatever the tag points at now. With a
`signature`, it loads them only if they are signed by that identity, verified offline against the
Sigstore public-good trusted root. `identity-pattern`, a regular expression matching the whole identity,
may replace `identity` ([spec §10.3](specs/component-interfaces/spec.md#103-oci-artifacts-and-integrity)).
A private registry's credential is the source's `auth`, written with `${VAR}` references.

### 10.4 What is recorded

Every contract a session writes, and every verification summary, lists the components it loaded: your
handshake's name and version, and where the bytes came from by digest, with the signer when one was
verified ([spec §10.3](specs/component-interfaces/spec.md#103-oci-artifacts-and-integrity)):

```json
"components": [ { "name": "yaml", "version": "1.0.0",
                  "source": { "kind": "oci", "reference": "ghcr.io/acme/janus-yaml:1.0.0", "digest": "sha256:…",
                              "signer": { "identity": "https://github.com/acme/…", "issuer": "https://token.actions.githubusercontent.com" } } } ]
```

In a contract it is under `metadata.components`. A `file` source is recorded by the SHA-256 of the file,
never its path.

## 11. What is not available yet

These are in the specification, and no engine calls them on a declared component yet. A handshake that
declares one fails to load, with a message naming it
([ADR 0104](decisions/0104-the-component-interface-is-generally-available-for-content-and-matcher-components-over-wasm.md)):

- **Generators** (`contributes.generators`, `generator/generate`): fresh values a mock or verifier fills in.
- **Variant dimensions** (an operator's `variant-facet`, `matcher/variant-space`).
- **Hooks** (the `hook` interface).
- **Transports** (the `transport` interface). A WebAssembly component has no sockets by design. Out-of-process
  components, which could, are not loaded yet either.

## 12. What stays stable

Within component protocol v1
([ADR 0104](decisions/0104-the-component-interface-is-generally-available-for-content-and-matcher-components-over-wasm.md)):
the WIT world never changes, and the schemas and their meaning change only by adding optional members,
operations and codes, which your component may ignore. A member you do not read today will never become
one you must. Your side of the bargain is [spec §12.2](specs/component-interfaces/spec.md#122-what-a-component-may-change):
once published, what an operator or action means is frozen. New behaviour is a new name, or a new major.

## 13. Checklist

- [ ] Every response echoes the request's `id`, with `ok` or `error`.
- [ ] Anything before `component/hello` is `handshake-required`. An `op` you do not implement is
      `operation-unsupported`.
- [ ] The handshake's name matches what a project will declare. Every contribution is in your namespace.
- [ ] Decode accepts `base64`, `text` and `json` slot values, and refuses octets that are not your type with
      `decode-failed`.
- [ ] `decode(encode(d)) == d`, members keep their order, and encode works with no `shape`.
- [ ] Losses your format cannot avoid are declared as degradations.
- [ ] No panics, no state between calls, no imports beyond what you were granted.
- [ ] Built for `wasm32-wasip2`, loaded by `janus explain --config` without `component-invalid`.
- [ ] Its conformance cases pass `janus component check`.
- [ ] Published, pinned by digest, signed, and declared with that digest and signer.
