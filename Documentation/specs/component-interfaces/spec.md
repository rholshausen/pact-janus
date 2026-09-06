# Component interfaces specification (v1, final)

Plan task: **2.6**. Status: **final**.

This document specifies the four interfaces the kernel loads behind it — **transport**, **content**,
**matcher** and **hook** — the pipe they speak, how a component declares what it contributes, how
built-in and third-party components are held to the same interface, and how an out-of-tree component is
named, distributed, sandboxed and resolved.

Its architecture is fixed by two decisions.
[ADR 0012](../../decisions/0012-one-interface-two-bindings.md) makes a component interface *the engine
protocol's frame shape turned around* — the engine calls, the component answers, with schema-governed
JSON documents over the same frozen byte-pipe — and gives that one interface two bindings: a native
binding for in-tree components and the byte-pipe binding for out-of-tree ones, held together by a
conformance corpus both must pass.
[ADR 0013](../../decisions/0013-component-hosting-is-an-embedding-capability.md) makes *hosting* a
declared capability of the running embedding, and settles distribution, integrity and sandbox grants.

The JSON Schemas under [`schemas/v1/`](schemas/v1/) are the **specified surface**; this prose defines
their semantics. They follow the Engine Protocol's open-world authoring rules
([protocol spec §2.2](../engine-protocol/spec.md)) in full and are enforced by the same CI checker —
these documents cross the same boundary and face the same version skew.

Evidence: spikes [1.4](../../../spikes/1.4-engine-hosting-plugins/FINDINGS.md) (hosting WASM components:
discovery, sandbox, traps, deadlines, cost), [1.5](../../../spikes/1.5-message-transport-shape/FINDINGS.md)
(the transport primitives, restated role-neutrally), [1.2](../../../spikes/1.2-wasm-embedding/FINDINGS.md)
(the engine as a guest, and the import surface), [1.3](../../../spikes/1.3-subprocess-embedding/FINDINGS.md)
(the stdio framing the out-of-process binding reuses, and the orphan-prevention property that comes with
it), [1.1](../../../spikes/1.1-idl-bakeoff/FINDINGS.md) (why the surface is documents and not types).

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [What a component is](#2-what-a-component-is)
3. [The component pipe](#3-the-component-pipe)
4. [Parts, slots and content](#4-parts-slots-and-content)
5. [The transport interface](#5-the-transport-interface)
6. [The content interface](#6-the-content-interface)
7. [The matcher interface](#7-the-matcher-interface)
8. [The hook interface](#8-the-hook-interface)
9. [Bindings](#9-bindings)
10. [Resolution, distribution and trust](#10-resolution-distribution-and-trust)
11. [Errors](#11-errors)
12. [Versioning and compatibility](#12-versioning-and-compatibility)
13. [Where full symmetry hurts](#13-where-full-symmetry-hurts)

Worked examples — the built-in HTTP transport driven through both bindings, and a third-party CSV
content component from handshake to plan fragment — live under [`examples/`](examples/), validated
against these schemas by `cargo test -p pact_janus_schema_compat`.

---

## 1. Scope and conformance

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in
RFC 2119.

Conformance roles:

- A **component** implements one or more of the four interfaces and answers the operations of each.
- The **engine** loads components, calls them, and is the only caller.
- A **loader** binds a component to the engine: the native, WASM and subprocess bindings of §9.

In scope: component identity and versioning, the pipe and its frames, the handshake and the
contribution vocabulary, the four interfaces' operations and documents, the three bindings and the
conformance rule that keeps them one interface, resolution and distribution, sandbox grants, error
shapes, and the evolution rules.

Out of scope, with owners:

| Question | Owner |
|---|---|
| the shape operators a component operator sits beside, and `admits` | design [2.2](../shape-language/spec.md) |
| the variant space a contributed dimension joins, and how variants are selected | design [2.3](../variant-semantics/spec.md) |
| the plan node grammar a contributed fragment is written in, and the action vocabulary | design [2.4](../plan-grammar/spec.md) |
| what a contract records about components it required | design [2.5](../contract-file/spec.md) |
| the frames between host and engine | design [2.1](../engine-protocol/spec.md) |
| **which** hook points exist, their ordering, config schema and secret handling | design 2.7 |
| the subsumption walk that consumes a comparability declaration | design 2.8 |
| CLI surface for listing, pinning and inspecting components | design 5.5 |

The division with design 2.7 is worth stating plainly because it is the least obvious one: **this
document owns the hook *interface*** — how a hook component is called, what it is handed, what it may
change — and **2.7 owns the hook *system***: which points exist, what configures them, how failures at
each point are treated, and how secrets reach them. A hook component is not the only implementation of
a hook point (2.7 also has `exec` and HTTP endpoints); it is the one that goes through this interface.

## 2. What a component is

### 2.1 Identity

A component has a **name**, a **version**, and implements one or more **interfaces**.

- `name`: `[a-z][a-z0-9-]*`. It is the component's identity and, critically, its **namespace** (§2.4).
- `version`: a semantic version string. Only the major is contract-relevant (§2.3).
- `interfaces`: a non-empty subset of `transport`, `content`, `matcher`, `hook`.

A component is a *package*, not a role: one component may implement several interfaces, and real ones
do — a protobuf component is a content handler *and* a matcher, because the operators it contributes
(`protobuf:enum`) only mean anything against documents it decoded.

### 2.2 The four interfaces

| Interface | Owns | Operations |
|---|---|---|
| `transport` | moving wire traffic in both directions, and mapping it to and from parts | §5 |
| `content` | turning octets into a document and back, for one or more content types | §6 |
| `matcher` | contributed shape operators, plan actions, generators, and comparability | §7 |
| `hook` | doing something at a named lifecycle point | §8 |

Matching and generating are deliberately **one interface, not two**. The shape language requires a
generator's output to be admitted by the shape it generates for (shape spec §3.4); keeping both halves
in one component is what lets the engine check that pair against a single declared authority rather than
hoping two components agree.

### 2.3 Versions and requirements

An interaction names what it cannot be matched without, as `<interface>/<name>` with a major version
floor (contract spec §7):

```json sketch
{ "requires": [ { "component": "content/protobuf", "min-version": 2 } ] }
```

- The requirement names a **role of a component**: "a component called `protobuf` that provides the
  `content` interface, major 2 or above".
- The comparison is against the version the **handshake declares** (§3.2), never against a tag, a
  filename or an OCI reference. What is loaded is what answers.
- Majors only. ADR 0002 made the boundary coarse-grained on purpose, and a contract that pins a patch
  is a contract that breaks on a bug fix it wanted.
- An engine MUST collect the union of requirements across all interactions **before** the run starts
  and fail unsatisfiable ones as `component-unavailable`, naming the requirement (§11.3). A run that
  discovers a missing component at interaction 40 of 50 has already wasted the information it had at
  the start.

### 2.4 The namespace rule

A component's contributed shape operators and plan actions are namespaced by its **name**, not by its
interface: `protobuf:enum`, `json:parse`, `http:status-class` (shape spec §3.5, plan grammar §4.1). Two
consequences, both normative:

- **Names are unique within a resolution scope**, whatever interfaces they implement. Two declared
  components with the same name is a configuration error (`component-conflict`, §11.2), detected at
  resolution and never at first use.
- **An unnamespaced operator or action name is never resolved to a component.** That rule belongs to
  designs 2.2 and 2.4; it is repeated here because it is the rule a component author is most likely to
  want to break, and the engine enforces it at handshake time: a component that declares an
  unnamespaced contribution, or one namespaced with a name that is not its own, fails to load
  (`component-invalid`).

Checking contributions against the component's own name at load time — rather than at use — is what
makes the namespace a real partition instead of a convention.

## 3. The component pipe

### 3.1 Frames

The pipe carries the Engine Protocol's frames (protocol spec §4), with the roles swapped: the **engine
sends `RequestFrame`s and the component answers with `ResponseFrame`s**, each carrying exactly one of
`ok` or `error`. Correlation ids, the `op` naming rule (`<area>/<verb>`), the malformed-frame rule and
the unknown-op rule are all the protocol's, unchanged.

Three rules are specific to this pipe:

- **The engine is the only caller.** A component never initiates a call into the engine. Everything it
  needs arrives in a request body. This is what makes the WASM binding possible at all — spike 1.4
  showed how loudly wasmtime polices instance re-entry — and it is why inbound wire traffic is polled
  rather than pushed (§5.3).
- **There are no event frames.** Streams and events belong to the host-facing protocol (protocol §9);
  the engine is their only producer. Component activity that a host should see — hook outcomes,
  transport progress — reaches the host as engine-emitted events built from operation results.
- **Calls are serial per instance.** A component MUST NOT assume it will be called concurrently, and
  MUST NOT require it. Concurrency is the engine's business, expressed as instances (§3.4).

An `op` this specification does not define, on an interface the component declared, MUST be answered
with `operation-unsupported` naming it — never silence, never a default.

### 3.2 The handshake

The first request on every component pipe is **`component/hello`**. Any other operation before it is
answered with `handshake-required`.

Schema: [`schemas/v1/handshake.schema.json`](schemas/v1/handshake.schema.json).

Request (`ComponentHello`), engine → component:

```json component-hello
{
  "component-protocol-versions": [1],
  "engine": { "name": "janus-engine", "version": "0.1.0" },
  "grants": { "env": [], "fs": [], "network": false },
  "capabilities": { }
}
```

Result (`ComponentHelloResult`), component → engine:

```json component-hello-result
{
  "component-protocol-version": 1,
  "component": { "name": "csv", "version": "1.2.0" },
  "interfaces": ["content", "matcher"],
  "contributes": {
    "content-types": [
      { "media-type": "text/csv", "degradations": [
        { "code": "numeric-lexical",
          "message": "CSV carries no numeric type; decimal and integer are indistinguishable." } ] }
    ],
    "actions": [ { "name": "csv:parse" } ],
    "operators": [
      { "name": "csv:column", "comparability": "exact" }
    ]
  },
  "capabilities": { "batch-apply": { } }
}
```

- The component picks the first version in `component-protocol-versions` it supports; if it supports
  none it MUST answer `protocol-version-unsupported` with `details.supported`, and the engine reports
  it as a load failure rather than continuing.
- `grants` tells the component what it was given, so it can fail its own handshake usefully rather than
  failing at first use — a component that needs the network and was granted none SHOULD say so here.
- `capabilities` on both sides is the protocol's open capability mechanism (protocol §5.3), used for
  optional operations: a component declaring `batch-apply` accepts more than one value per
  `matcher/apply` call; one that does not gets one value per call.

**The handshake is the only source of truth about what a component provides.** There is no manifest to
agree with it (ADR 0012) — an artifact's metadata may hint at contributions for cataloguing, but the
engine indexes what the running component declared and nothing else.

### 3.3 Contributions

`contributes` is an object of open vocabularies; a component populates the members its interfaces make
sense of, and the engine ignores members it does not know.

| Member | Declared by | Each entry carries |
|---|---|---|
| `transports` | `transport` | `kind` (the open transport vocabulary: `http`, `message`, …), `roles` (`serve`, `drive`) |
| `content-types` | `content` | `media-type` (a media type or a `type/*` pattern), optional `degradations` (§6.4) |
| `actions` | `matcher`, `content` | `name` (namespaced), optional `lazy` (plan grammar §2.4) |
| `operators` | `matcher`, `content` | `name` (namespaced), `comparability` (§7.5), `variant-facet` (§7.4) |
| `generators` | `matcher` | `name` (namespaced), optional `produces` |
| `hook-points` | `hook` | `point` (design 2.7's vocabulary), optional `changes` (§8.2) |

Every declared name MUST be namespaced with the component's own name (§2.4). Every entry MAY carry
additional members; the engine ignores what it does not know, and a component MUST NOT depend on an
engine reading an entry member this specification does not define.

### 3.4 Instances, lifetime and deadlines

An **instance** is one live copy of a component. The engine names instances; a component never invents
an identifier. All of these are normative engine behaviour, following spike 1.4 §3:

- **One instance per session by default.** Sessions are the only resource (protocol §7.1) and that rule
  extends inward: a component MUST NOT retain state across sessions, and state that outlives a session
  is a bug in the component. The engine MAY use an instance per call — spike 1.4 measured 5.8 µs with
  `InstancePre`, which is 0.4 % of decoding one 100 KB document — and a component MUST behave
  identically either way.
- **Every call has a deadline.** The engine MUST bound every component call in time (epoch interruption
  for the WASM binding, an engine-side timer with kill escalation out of process) and MUST surface
  expiry as an error value
  (§11.2). A hung component cannot hang a verification.
- **A trap poisons the instance, loudly.** After a trap, timeout or panic the engine MUST NOT reuse the
  instance; it recreates one (spike 1.4: 0.1 ms) and reports the failure as a value. Continuing on a
  half-dead instance is the failure mode this rule exists to make impossible.
- **`component/shutdown`** (body `{}`) is offered before an instance is dropped where the binding can
  deliver it. A component MUST NOT *require* it: an instance may vanish without notice, and anything
  that must be released — a listening socket, a broker subscription — is released by the interface
  operation that owns it (`transport/stop`) or by the process exiting.

### 3.5 Errors are values here too

A component answers a failure with a `ComponentError` in the frame's `error` member. It never traps,
panics, exits or hangs by design; when it does anyway, the binding converts that into a
`ComponentError` on its behalf (§11.2), so the engine's caller sees one shape. Full taxonomy in §11.

## 4. Parts, slots and content

Three documents cross every interface in this specification, and they are the same documents design 2.5
records and design 2.2 attaches shapes to. Schema:
[`schemas/v1/parts.schema.json`](schemas/v1/parts.schema.json).

**Parts** is a map of part name to part. **A part** is a map of slot name to slot value. **A slot value**
is design 2.5's `SlotValue` — a wrapper carrying `content`, tagged by `encoded` (`json`, `text`,
`base64`), with an optional `content-type`.

```json parts
{ "request": {
    "method":  { "content": "POST" },
    "path":    { "content": "/orders" },
    "headers": { "content": { "content-type": ["application/json"] } },
    "body":    { "content": "eyJpZCI6IDF9", "encoded": "base64", "content-type": "application/json" } } }
```

Normative points:

- **Part and slot names are the transport's and content component's business**, never the kernel's
  (shape spec §3.6, contract spec §5.1). This specification fixes the nesting and nothing else. There
  is no `request`/`response` pair anywhere in it, because a message interaction has neither.
- **Transports carry octets, not documents.** A transport puts a body slot's bytes in `content` with
  `encoded: "base64"` and the declared media type in `content-type`; turning those into a document is
  the content component's job and happens later (spike 1.5 finding 7a). A transport that parses bodies
  has taken a content component's job and will disagree with it eventually.
- **The wrapper is unconditional.** A bare value with a sibling tag would be ambiguous against user data
  that happens to carry that member — the trap contract spec §5.3 refuses, refused identically here.
- **An endpoint descriptor is an open document** (`EndpointDescriptor`): host and port for HTTP, broker
  and topic details for messaging, whatever else a transport needs. It is never assumed to be a URL
  (spike 1.5 finding 1), and the engine passes it to the host unchanged as the `endpoint` of
  `consumer-session/start-transport` (protocol §8.2).

## 5. The transport interface

Schema: [`schemas/v1/transport.schema.json`](schemas/v1/transport.schema.json).

### 5.1 The primitives

Spike 1.5 restated the RFC's transport sketch in role-neutral terms after finding that "start a mock
endpoint / drive requests at a provider" is HTTP-accented and does not survive messaging literally.
What survives is five operations plus a disposition, and every scenario in that spike — HTTP mock, HTTP
verification, message publish, message consume, sync-message both ways — is a composition of them:

| Operation | Body → Result |
|---|---|
| `transport/start` | `{ instance, kind, role, options? }` → `{ endpoint }` |
| `transport/stop` | `{ instance }` → `{ }` |
| `transport/send` | `{ instance, parts, await-reply?, timeout-ms? }` → `{ reply? }` |
| `transport/poll-inbound` | `{ instance, timeout-ms? }` → `{ inbound? }` |
| `transport/reply` | `{ instance, event, parts }` → `{ }` |
| `transport/dispose` | `{ instance, event, disposition }` → `{ }` |

`instance` is engine-assigned (§3.4) and carried by every operation. `role` is `serve` (the component
accepts inbound traffic) or `drive` (it initiates outbound traffic); a transport declares which roles
it supports per kind in its handshake, and MAY support both.

### 5.2 Sending

`send` delivers `parts` and, when `await-reply` is true, waits for the reply and returns it as `reply`.

**Both halves of the reply are optional, and that is the finding, not a convenience.** Fire-and-forget
publication has no response to match; a message-consumer test's whole outbound act is a `send` with no
reply. A transport that cannot honour `await-reply: true` for its kind MUST answer
`operation-unsupported` rather than returning an empty reply, because "no reply expected" and "reply
expected but absent" are different observations about a provider.

### 5.3 Receiving

`poll-inbound` returns the next arrived traffic, or nothing if none arrived within `timeout-ms`. The
engine drives; the transport never calls in.

```json poll-inbound-result
{ "inbound": { "event": "e-7", "parts": { "request": { "method": { "content": "GET" } } },
               "expects-reply": true } }
```

- `event` is the transport's identifier for this arrival, opaque to the engine, and the one piece of
  state this interface carries (spike 1.5 finding 5). It exists so `reply` and `dispose` can address a
  specific arrival and so results can be attributed to one.
- `expects-reply` says whether this arrival has a reply half at all. HTTP requests do; a consumed
  broker message does not.
- **Correlation is wire business.** Reply topics, correlation ids, partitions and consumer groups stay
  inside the transport; the engine sees parts in and parts out for every kind alike. That is what makes
  an HTTP mock and a sync-message mock literally the same engine loop.

`reply` completes an arrival that has a reply half. `dispose` completes one either way, with a
`disposition` from an open vocabulary — `accept`, `reject`, `release` — and it exists because spike 1.5
found the gap: an in-memory broker can drop an unwanted message silently, a real one must acknowledge,
negative-acknowledge or release it for redelivery, and an interface with no way to say which will grow
a broker-shaped hole in Phase 8. The engine MUST dispose of every arrival it polls; a transport whose
kind has no such concept ignores it.

### 5.4 What the transport does not decide

- **Passive vs emissive is not the transport's.** Whether `consumer-session/serve-variant` means "arm
  and wait" or "publish now" is a property of the interaction's transport binding, decided in the
  protocol layer (protocol §7.4, contract §4.3). The primitives are the same either way: passive is
  `poll-inbound` → match → `reply`, emissive is `send`. Spike 1.5 finding 3 is explicit that this
  distinction lands in the protocol, not here, and putting it here would give every transport an
  opinion about test semantics.
- **Matching is not the transport's.** It carries parts; the kernel matches them.
- **Content is not the transport's** (§4).

## 6. The content interface

Schema: [`schemas/v1/content.schema.json`](schemas/v1/content.schema.json).

| Operation | Body → Result |
|---|---|
| `content/decode` | `{ content-type, value, options? }` → `{ document, degradations? }` |
| `content/encode` | `{ content-type, document, options? }` → `{ value }` |
| `content/compile` | `{ content-type, shape, path }` → `{ fragment? }` |
| `content/detect` | `{ value, hint? }` → `{ media-type?, confidence? }` |

`value` in `decode` and `encode` is a `SlotValue` (§4): octets in, octets out.

### 6.1 Decoding is the bytes boundary

`decode` turns a slot's octets into a **document in the shape language's model** (shape spec §2.1) —
JSON values plus bytes — and the shape then applies to *that document*. This operation is the entire
content of the rule that the kernel knows nothing about JSON.

A component that cannot decode answers with an error (§11.1), and the engine fails the interaction. It
never applies a shape to a guess: shape spec §2.2 makes that normative, and this is the operation where
it is enforced.

### 6.2 Encoding is how variants become traffic

`encode` is decode's inverse, and it exists because a consumer test must *produce* bodies: each selected
variant's document becomes the octets a mock serves (plan 4.3/4.4). `decode(encode(d))` MUST equal `d`
for every document the component's own `decode` can produce. Round-tripping is not a nicety here —
a contract records both the shape and a produced example, and a component whose two halves disagree
records an example its own matcher would reject.

### 6.3 Contributing plan fragments

`compile` is asked for a body slot's shape and MAY return a **plan fragment** — a plan document in
design 2.4's grammar, spliced at `path` — so that the component's own decoding and addressing appear in
the plan the user can `explain`. Returning nothing is legitimate: the kernel then compiles the slot
generically and calls `decode` at execution time.

The fragment is written in the plan grammar and constrained by it: it may use core actions and the
component's own namespaced actions, and nothing else. Grammar-version targeting and what happens when
the engine's grammar moves are §12.3, and are what task 8.4 stresses.

### 6.4 Declaring degradations

Some content types cannot express distinctions the shape language can make. A CSV has no numeric type;
a binary encoding may have one numeric type where JSON has two lexical forms. Shape spec §4.2 requires
the component to **say so rather than pretend**, and this is where it says it:

- **Statically**, in the handshake (§3.2): a `degradations` entry per content type, each with a code
  from an open vocabulary and a message. `numeric-lexical` — `decimal` degrades to `number` — is the
  worked case.
- **Per decode**, in `decode`'s optional `degradations`, for value-dependent losses, each with the path
  where the loss occurred.

An engine SHOULD surface declared degradations in `explain` output and in verification results. A
component that silently narrows what it can distinguish makes a contract weaker than it reads.

### 6.5 Detection

`detect` answers "do these octets look like a type you handle?", for the shape language's `content-type`
operator — the one operator that inspects octets rather than a decoded document (shape spec §4.2).
It is optional; a component that does not implement it answers `operation-unsupported`, and the engine
falls back to the declared media type. Detection MUST NOT be used to *override* a declared content type,
only to resolve its absence: a provider that declares the wrong type is a finding, not a thing to
silently correct.

## 7. The matcher interface

Schema: [`schemas/v1/matcher.schema.json`](schemas/v1/matcher.schema.json).

| Operation | Body → Result |
|---|---|
| `matcher/compile` | `{ operator, node, path }` → `{ fragment }` |
| `matcher/apply` | `{ action, config?, values }` → `{ results }` |
| `matcher/variant-space` | `{ operator, node, path }` → `{ dimensions }` |
| `matcher/compare` | `{ operator, provider, consumer }` → `{ verdict, reason? }` |
| `generator/generate` | `{ generator, config?, context? }` → `{ value }` |

`generator/generate` carries its own operation area while belonging to this interface: an op name says
what the call *is*, and an interface says who implements it. They are one interface for the reason §2.2
gives — a generator's output must be admitted by the shape it generates for, and one declared authority
is what lets the engine check that.

### 7.1 Two stages, mirroring the kernel's own

A contributed operator behaves exactly as a core one does: it is **compiled** into plan nodes, and the
nodes are **executed**. `compile` turns one shape node into a fragment; `apply` executes one of the
component's actions against values at run time. Nothing about a component operator is special-cased in
the interpreter — that is the whole point of plan grammar §4.6, and it is why `json:parse` is a
component action in a spec that has no JSON in the kernel.

An operator whose `compile` returns a fragment using only core actions needs no `apply` at all. That is
the cheapest kind of component operator and worth preferring: it costs nothing at match time and its
behaviour is fully visible in `explain`.

### 7.2 Batching

`values` is an array and `results` is an array of the same length, in the same order. This is in the
interface from day one because spike 1.4 measured the pipe binding at 1.4 µs per call: fine per value at
test scale, and the obvious lever if it ever is not. A component MAY declare the `batch-apply`
capability to accept more than one value per call; without it the engine sends one, and the arrays are
of length 1. The native binding pays neither cost, which is exactly the asymmetry §9.4 makes visible.

Each result is a **plan result document** in design 2.4's result form (`ok`, `value`, `error`) — the
same shape a kernel action produces, so a mismatch from a component is a mismatch, not a special case,
and reaches the user through the same executed plan (spike 1.4 §2).

### 7.3 Generating

`generator/generate` produces a value where one must be fresh rather than replayed (shape spec §3.4).

**A generator's output MUST be admitted by the shape it generates for.** The engine SHOULD check this
and MUST report a violation as a component error, never as a test failure: a generator producing a value
its own shape rejects is a broken component, and reporting it as a failed interaction sends the user to
debug the wrong thing.

`context` carries what the generator is allowed to know — the variant assignment, values already
produced in this interaction — and its members are an open vocabulary. A generator that needs ambient
state (a clock, randomness) gets it from its own runtime under its grants (§10.4), not from the engine.

### 7.4 Contributing variant dimensions

`matcher/variant-space` lets a component operator contribute dimensions to the variant space (shape spec
§6.6, variant semantics §2.1). The returned dimensions are that schema's documents, with two obligations
this interface adds:

- **Point order is minimal-to-maximal.** Variant semantics §3.1 records a component facet's extremes as
  "as the component declares"; this is that declaration, and it needs no extra member — the first point
  is the minimal, the last is the maximal, and the boundary variants take them.
- **Gates must form a forest.** Variant semantics §2.1 notes that no core shape can produce a cycle but a
  component operator could. An engine MUST reject a contributed dimension set whose gates are not a
  forest as `interaction-invalid`, naming the operator.

An operator that contributes no dimension returns an empty list; that is the common case and the default.

### 7.5 Declaring comparability

The subsumption checker (design 2.8) asks `admits(P) ⊆ admits(C)` and must answer `yes`, `no` or
`unknown` (shape spec §8). For a component operator it can answer nothing on its own beyond the identity
floor. So an operator declares a **comparability class** in the handshake:

| Declared | The checker may | And |
|---|---|---|
| `exact` | decide `yes`/`no` by calling `matcher/compare` | the component MUST implement `compare` |
| `conservative` | answer `yes` on identity and on exactly-wider containers; `unknown` otherwise | `compare` optional |
| `opaque` (or nothing declared) | answer `yes` on identity, `unknown` otherwise | — |

`compare`'s `verdict` is one of the same three values. A component MUST answer `unknown` rather than
guess — shape spec §8's rule that a wrong `yes` is how a checker loses its users applies to components
verbatim, and a component is the party most tempted to be optimistic about its own operator.

## 8. The hook interface

Schema: [`schemas/v1/hook.schema.json`](schemas/v1/hook.schema.json).

| Operation | Body → Result |
|---|---|
| `hook/invoke` | `{ point, context, config? }` → `{ outcome, changes?, data? }` |

### 8.1 What a hook is handed

`point` is design [2.7](../lifecycle-hooks/spec.md)'s open vocabulary (`before-verification`,
`state-setup`, `before-request`, `produce-message`, `consume-message`, `after-response`,
`state-teardown`, `after-verification`, …). `context` is that design's `HookContext` — an open document
assembled by the engine for the point, carrying the parts in play, the interaction reference, the
variant assignment and the provider state with its parameters — and a component receives exactly what a
script, a command or an HTTP endpoint receives at the same point. `config` is the hook's own configuration from the project config,
already interpolated (2.7 owns interpolation and secret handling; a component receives values, never
templates).

### 8.2 What a hook may change

`outcome` is `ok`, `failed`, `skipped` or — at design 2.7's state points — `unsupported`. `changes` is a map of context slot to replacement value, and
it is governed by declaration, not by trust:

- A hook component declares in its handshake which context members it may change, per point.
- The engine applies only declared changes and MUST reject an undeclared one as `component-failed`,
  naming the member.

Declaring mutation up front is what makes a hook chain reviewable: a config lists which hooks run at a
point, and each hook's declaration says what it can touch, so "which hook rewrote this header" is
answerable from the configuration rather than by bisecting a run. A hook that only observes declares no
changes and cannot make any.

`data` is opaque hook output surfaced to the host in the `verification/hook` event (protocol §9.6). A
`failed` outcome is a *hook* failure, whose treatment — abort the run, fail the interaction, warn — is
design 2.7's ordering-and-failure semantics, not this interface's.

## 9. Bindings

One interface, three ways of reaching it (ADR 0012). A binding changes how a document travels, never
what the document is: **no operation exists in one binding and not another**, and no operation carries
different members depending on how it is reached.

### 9.1 The native binding (in-tree)

An in-tree component is compiled into the engine and invoked through a Rust trait per interface whose
methods take and return the *same documents as in-memory values*, with no serialisation. It is a
projection of the frame surface, and the projection is specified rather than incidental:

- **One method per operation**, named for the operation, taking that operation's request document and
  returning `Result<ResultDocument, ComponentError>`.
- **The handshake happens too.** An in-tree component declares its contributions the same way, is
  indexed the same way, and is subject to the same namespace check (§2.4). The engine has no privileged
  path to a built-in's capabilities.
- **Errors are values here too.** A built-in that panics is an engine bug, and the dispatch boundary
  converts the panic to a `ComponentError` exactly as the WASM binding converts a trap. The kernel does
  not distinguish where a component error came from.
- **No back door.** An in-tree component MUST NOT reach into engine state that the frames do not carry.
  The conformance run (§9.4) is what makes this checkable rather than merely asked for.

### 9.2 The WASM binding (out-of-tree, default)

The component exports the frozen byte-pipe world, the same shape ADR 0003 froze for the engine itself:

```wit
call: func(request: list<u8>) -> list<u8>;
```

One request frame in, exactly one response frame out, strictly serial per instance, UTF-8 JSON with no
framing header — the buffer length delimits the frame. There is no negotiated encoding on this pipe in
v1 and no need for one: both parties are the same process.

Normative host obligations, all measured in spike 1.4:

- **Deny by default.** A default WASI context grants nothing: no preopens, no environment, no sockets.
  Grants (§10.4) are explicit additions.
- **Governed imports.** A component's imports MUST be checked against its grants at load; surplus
  imports are a load failure. A component built with `std` will import WASI interfaces it never uses,
  and satisfying them with nothing is why that is harmless — but "harmless" is a property to verify,
  not to assume (spike 1.4 finding 7).
- **Bounded execution.** Epoch interruption with a per-call deadline (§3.4).
- **Loud poisoning.** A trapped instance is never reused (§3.4).

### 9.3 The subprocess binding (out-of-process, escape hatch)

A component that needs ambient capability a sandbox exists to deny — raw sockets, a long-lived server,
an existing native client library — runs as a separate process, spawned by the engine, speaking the
same frames over **the Engine Protocol's own stdio framing** (protocol §3.2): a `Content-Length` header
section, then exactly that many bytes of frame; unknown headers ignored; stdout carries protocol frames
and nothing else; logs to stderr.

The RFC names gRPC here, and this specification deliberately does not follow it. Spike 1.3 already
specified, measured and validated this framing for the engine's own subprocess pipe, and three of its
findings transfer to a component process unchanged:

- **The framing client is 30–60 dependency-free lines per language** (finding 6). A component author in
  Go, Java or Python writes a loop, not a service definition, and needs no protobuf toolchain to
  implement an interface whose documents are JSON. gRPC would put a code-generation step between an
  author and their first working component — on the extension path, which is the one path where
  friction is the whole cost.
- **Orphan prevention comes free, as a design property rather than platform code** (finding 2). The
  component treats stdin as its lease on life and MUST exit on EOF, so a killed engine — or a killed
  test runner above it — cannot leave component processes behind. Over gRPC that property has to be
  rebuilt out of process groups, job objects or heartbeats, per platform.
- **Length-prefixed framing is self-synchronising** (finding 6): a malformed body is reported in-band
  and the stream stays in sync, which is exactly the behaviour §11.2 needs when a component starts
  producing garbage rather than failing cleanly.

Normative rules:

- The engine spawns the process from the declaration's `source` and MUST bound every call in time,
  escalating to a kill when a deadline passes (§3.4).
- The component MUST exit on stdin EOF, and MUST NOT write anything but frames to stdout.
- Correlation is by frame id, as everywhere else. **One process MAY host several instances** — every
  operation carries its `instance` (§3.4) — so a component with expensive start-up is not obliged to
  be one process per instance.

Two things this binding does not have, stated plainly rather than implied:

- **Grants are not enforceable.** The component runs with the engine user's authority. Declaring grants
  for it documents intent and nothing more, and this specification says so because a sandbox that is
  announced but absent is worse than one that was never claimed.
- **It is the escape hatch, not the second default.** It exists for the cases WASM cannot serve.
  Task 8.3 proves the boundary exists; nothing else should reach for it.

On interoperability with today's pact-plugins: what is retained is the *architecture* — an
out-of-process component the engine drives — and not the wire, because the frames differ completely
either way. Keeping gRPC would have bought the transport and not the protocol, at the cost of a second
framing, a code-generation dependency, and an orphan problem this project has already solved. If an
existing gRPC plugin ecosystem ever needs bridging, a shim process speaking both is a smaller thing
than a second specified pipe. `grpc` remains available as a loader name should evidence ever demand it
— the vocabulary is open (§10.1) — but v1 specifies one out-of-process pipe.

### 9.4 Dual-binding conformance

This is the mechanism that makes "built-ins implement exactly these interfaces" a fact rather than an
intention (ADR 0012 decision 4).

A **component conformance corpus** is a set of cases, each a sequence of `(op, body)` calls with
expected results, expressed in the same documents as the frames. Schema:
[`schemas/v1/conformance-case.schema.json`](schemas/v1/conformance-case.schema.json). Rules:

- Every in-tree component MUST be runnable through the WASM (or a serialising in-process) binding as
  well as the native one, in tests.
- CI runs the corpus through both bindings and requires **identical results** after canonicalisation.
- A case that can only be expressed in one binding is not a conformance case; it is a report that the
  interface has split, and ADR 0012's tripwire (a) fires.

The corpus grows with the golden corpora of plan task 3.7 and is the second thing a third-party
component author runs after reading this document — the first being the handshake.

## 10. Resolution, distribution and trust

### 10.1 Hosting is a capability

Per [ADR 0013](../../decisions/0013-component-hosting-is-an-embedding-capability.md), the engine
declares which loaders the running embedding has, in the `engine/hello` result (protocol §5.3):

```json capability
{ "components": { "loaders": ["in-tree", "wasm", "subprocess"] } }
```

The WASM-guest embedding declares `["in-tree"]` — an engine that is itself a WASM component cannot host
WASM components, because there is no runtime with code generation inside a `wasm32-wasip2` guest. A host
learns this from the handshake rather than from a failure at interaction 40. A requirement that no
available loader can satisfy is `component-unavailable`, naming the requirement and the loaders present
(§11.3).

### 10.2 Declaration

Components are **declared**, never discovered. An engine that goes looking for something to satisfy
`content/protobuf` is an engine whose runs are not reproducible. Schema:
[`schemas/v1/component-config.schema.json`](schemas/v1/component-config.schema.json).

```json component-config
{ "components": [
    { "name": "csv",
      "source": { "kind": "oci", "reference": "ghcr.io/pact-foundation/janus-csv:1.2.0",
                  "digest": "sha256:9f2c…" },
      "grants": { "env": [], "fs": [], "network": false },
      "limits": { "deadline-ms": 10000 } } ] }
```

`source.kind` is an open vocabulary: `oci`, `file` (a local `.wasm`, for development and for task 8.1),
`subprocess` (a command the engine spawns). Where this configuration file lives and how it merges with the rest
of a project's configuration is design 2.7's business — it shares a file with hook configuration — and
this specification owns only the component entries.

### 10.3 OCI artifacts and integrity

An out-of-tree WASM component is distributed as an OCI artifact: an artifact type identifying it as a
Janus component, a config blob carrying its name and version, and one layer carrying the `.wasm`.
Resolution is:

1. resolve the reference (registry, repository, tag or digest);
2. if a `digest` is declared, the resolved manifest digest MUST match it, or the load fails —
   before any bytes are instantiated;
3. cache content-addressed by digest, so a second run of the same pinned component fetches nothing;
4. instantiate, handshake, index contributions (§3.2), check the namespace rule (§2.4);
5. check declared component versions against requirements (§2.3).

A declaration with a tag and no digest resolves the tag; CI SHOULD pin digests, because a tag is a name
and a digest is the artifact. The engine records the resolved name and version of every component that
took part in a run; a contract's `metadata.writer` map (contract spec §3) is where that lands for the
consumer side.

Signature verification is not specified in v1. It belongs on top of digest pinning rather than instead
of it, and the prototype has no evidence about which scheme the ecosystem would adopt; adding it later
is additive.

### 10.4 Grants

Grants are deny-by-default and per component:

| Grant | Meaning |
|---|---|
| `env` | environment variable names the component may read (never a wildcard in v1) |
| `fs` | directories preopened for it, each with `read` or `read-write` |
| `network` | whether outbound sockets are granted at all |

An engine MUST NOT grant what a declaration does not list, MUST fail the load when a component's
imports exceed its grants (§9.2), and MUST report a denied capability as the component's own error
rather than as a matching failure. Grants are a property of the *project's* configuration, not of the
component's request: a component asking for more is information for the human reviewing the config.

## 11. Errors

### 11.1 `ComponentError`

Schema: [`schemas/v1/component-error.schema.json`](schemas/v1/component-error.schema.json). The shape
is the Engine Protocol's `EngineError` — `code`, `category`, `message`, `details` — with a
component-scoped code vocabulary, because the kernel passes it through verbatim (protocol §10.2) and a
second error shape at that boundary would only need translating.

| Category | v1 codes |
|---|---|
| `protocol` | `malformed-frame`, `handshake-required`, `operation-unsupported`, `protocol-version-unsupported` |
| `document` | `unsupported-content-type`, `decode-failed`, `encode-failed`, `unknown-operator`, `unknown-action`, `invalid-config` |
| `component` | `transport-failed`, `hook-failed`, `capability-denied`, `unavailable` |
| `internal` | `internal` |

`details` conventions: `operation-unsupported` carries `op`; `unknown-operator`/`unknown-action` carry
the name; `decode-failed` carries `content-type` and, where it has one, a `path`; `capability-denied`
carries the grant it wanted.

### 11.2 When the component produces no error

A trap, a timeout, a crashed process or a panic in an in-tree component is still a failure that must
reach the caller as a value. The binding synthesises a `ComponentError` on the component's behalf, and
marks it: `source: "engine"`, with `code` one of `component-trapped`, `component-timeout`,
`component-exited`. The engine **MUST NOT translate** an error the component *did* produce — protocol
§10.2 is explicit that the kernel does not understand component error interiors — and MUST distinguish
the two cases, because "the component said no" and "the component died" send a reader to different
places.

### 11.3 At the engine boundary

Every component failure surfaces to the host as one of the protocol's two component codes:

| Situation | Engine error |
|---|---|
| a requirement no declared component satisfies, or no loader can load | `component-unavailable`, `details.component` naming the requirement, `details.loaders` naming what the embedding has |
| a namespaced operator or action with no component | `component-unavailable` (shape spec §3.7) |
| two declared components with the same name | `component-unavailable`, code `component-conflict` in `details` |
| a component that declared a contribution it may not (§2.4) | `component-unavailable`, `details.reason` = `component-invalid` |
| a component that answered with an error | `component-failed`, `details.error` = the `ComponentError` verbatim |
| a component that trapped, timed out or exited | `component-failed`, `details.error` = the synthesised error (§11.2) |

A component mismatch — a value the component's own matcher rejected — is **not** an error. It is a plan
result (§7.2) and travels as a mismatch, exactly as a kernel action's would.

## 12. Versioning and compatibility

### 12.1 Three versions, deliberately separate

| Version | Names | Changes when |
|---|---|---|
| the **component protocol** version | the frames and operations of this document | never, within v1; additively per protocol §11.2 |
| a **component's** version | one component's own releases | its author says so; majors are what contracts pin (§2.3) |
| the **plan grammar** version | the node grammar a fragment is written in | design 2.4 owns it |

Conflating any two of these is how a plugin ecosystem seizes up: an engine upgrade that forces every
component to re-release, or a component release that silently changes what a recorded contract means.

### 12.2 What a component may change

The engine protocol's additive-evolution rules (protocol §11.2) apply to these schemas unchanged, and
so does one rule from the shape language and plan grammar, restated here because it binds *component
authors* rather than this project:

- **A contributed operator's or action's semantics are frozen once published.** New behaviour is a new
  name. An operator's `admits` is recorded in contracts that will be read after everyone who wrote them
  has moved on (shape spec §9, plan grammar §7.1); redefining it silently changes what those contracts
  mean.
- **Contributions may be added**, and a component's declared list growing is not breaking.
- **A contribution may be deprecated but not removed** within a major version.

### 12.3 Fragments and grammar skew

A contributed plan fragment is authored against a plan-grammar version and shipped separately from the
engine — the one place a plan document crosses a version boundary (plan grammar §7.1). A fragment
therefore declares the grammar version it targets, and an engine whose grammar has moved on either
accepts it (the grammar grew additively, which is the designed-for case) or fails the load naming the
skew. It MUST NOT silently reinterpret a fragment written against an older grammar.

Task 8.4 is the stress test for exactly this, and it is scoped here rather than left to be discovered:
the question it answers is what happens when a component targets grammar v0 and the engine has moved.

## 13. Where full symmetry hurts

ADR 0012 committed the prototype to interface symmetry with two bindings, and the RFC asked for the
report, not just the decision. The report cannot be written yet — it needs 3.8, 4.2 and 8.1 — so what
this section fixes is **what must be measured and where each measurement lands**, so the Phase 9
write-up is an accumulation rather than an archaeology.

| Prediction | Evidence available now | Measured by |
|---|---|---|
| Per-body serialisation is the pipe binding's real cost, and content components pay it most | 2.6 ms per 100 KB round trip, double-ended serde (spike 1.4 §2) | 4.2 (JSON content through both bindings), 9.1 |
| Per-value dispatch is affordable at test scale; batching is the lever if it is not | 1.4 µs per plugin matcher call, ~700 K/s (spike 1.4 finding 3) | 4.2, 9.1 |
| Transports are the interface least suited to the WASM binding — they want sockets and long-lived servers | spike 1.5's HTTP transport was internally threaded; spike 1.2's engine wanted no sockets at all | 4.2, 8.3 |
| The primary embedding cannot host out-of-tree components at all | spike 1.2 (engine as guest) + spike 1.4 (hosting needs wasmtime) | ADR 0013; revisited in 9.2 |
| Keeping the in-tree JSON and HTTP components honest costs a maintained second binding | — | 9.4's conformance run; count the divergences it catches |
| "Documented by reading the core" is true only if a third party never needs engine source | — | 8.1, which is instructed to track every such moment |

The honest summary this design already supports: **interface symmetry is achievable day one and
packaging symmetry is not** — not for cost reasons, but because the embedding the project made primary
cannot host anything. A design that had committed to packaging symmetry would have discovered that in
Phase 4 with the HTTP transport half-written. Recording it here, with the loaders capability and the
conformance corpus as the two mechanisms that carry the consequences, is what task 2.6 owes the RFC's
question.
