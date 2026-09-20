# Findings for Phase 9

A running list of things found while *using* the prototype that belong to Phase 9's review (plan
§12) rather than to the task that turned them up: gaps between what v1–v4 did and what Janus can say,
design questions a fix would have to answer first, and anything else task 9.2's "what we learned" or
task 9.4's staged plan should not have to rediscover. Each entry says what was observed, how to
reproduce it, what the current code and specs do about it, and the options — not a decision. A
decision that comes out of the review lands as an ADR, and the entry links to it.

## 1. Media-type header values: a consumer's `application/json` must accept `; charset=utf-8`

**Found:** 2026-09-18, comparing the plan of the order-service v3 pact with the plan of the contract
`janus upgrade` produced from it. **Status:** open — reported by the upgrade as a `judgement` finding,
not fixed.

### What v1–v4 do, and why

A consumer test author writes the content type they depend on:

```json sketch
"headers": { "Content-Type": "application/json" }
```

and the provider answers `content-type: application/json; charset=utf-8`. That must **pass**: the
author said "JSON" and the provider sent JSON; the charset is a detail the author did not constrain,
and a framework or proxy on the provider side adding it is not a contract break. But if the author
*did* constrain it — `application/json; charset=utf-8` — then a provider sending
`application/json; charset=iso-8859-1` must **fail**, because the author named that parameter and the
provider sent a different value for it.

v1–v4 encode exactly this as their default header comparison. The legacy compiler emits it as
`match:header-value` (`engine/kernel/src/plan/legacy.rs`, `compile_headers`), a legacy-only core action
(plan-grammar spec §4.4) implemented in `engine/kernel/src/plan/interpret.rs` (`header_values_match`):

- a MIME-shaped value compares the base type case-insensitively, and every parameter named in the
  **expected** value must be present in the actual one with a case-insensitively equal value. Extra
  actual parameters, and a different parameter order, do not fail it (pact specification test cases
  `matches content type with charset`, `... with parameters in different order`, `content type
  parameters do not match`);
- a comma-separated value tolerates whitespace around the commas (`whitespace after comma different`);
- anything else is exact, case-sensitive equality (`header value is different case`).

### What the upgraded contract does

The shape language has no operator for that comparison, so the upgrade writes a header value with no
matching rule as `equality` — and each header as a list of lines, because the HTTP transport keeps
repeated header lines as separate entries. The same header compiles to:

```text
pact plan                                   janus plan
:"$.Content-Type" (                         :"$.content-type" (
  %match:header-value (                       %expect:count ( $.response.headers.content-type, 1 ),
    $.response.headers.content-type,          :"$.content-type[0]" (
    'application/json'                          %match:equality (
  )                                               $.response.headers.content-type[0],
)                                                 'application/json' ) ) )
```

Run against a provider response of `content-type: application/json; charset=utf-8`:

```text
pact:   %match:header-value (… 'application/json; charset=utf-8', 'application/json') => BOOL(true)
janus:  %match:equality     (… 'application/json; charset=utf-8', 'application/json')
          => ERROR(Expected 'application/json; charset=utf-8' to equal 'application/json')
```

The upgrade already says so — `rule-narrowed` (`judgement`) at the response headers, "the shape
language has no operator for that comparison yet" (`engine/kernel/src/upgrade.rs`, `headers_slot`).
It is `judgement` rather than `lossy` because the contract is *stricter* than the pact: it produces
false failures, never false passes. That is the safe direction, and it is still wrong for the
commonest header there is. Three differences, in decreasing order of how often they will bite:

1. **Unconstrained parameters fail.** The charset case above. A provider whose framework appends
   `; charset=utf-8` — most of them — fails every interaction that names a content type.
2. **List whitespace fails.** `a, b` against `a,b`.
3. **Repeated header lines fail.** `expect:count 1` rejects a provider that sends the header on two
   lines, which v1–v4 would have seen as one comma-joined value.

The sample provider happens to send exactly `application/json`, which is why the order-service
verification passes both ways and the gap only showed up by reading the two plans side by side.

### Reproduce

```sh
janus upgrade samples/order-service/pacts/web-app-order-service.json --out contract.janus.json
# values files: the order-service response, with the header set to
#   "application/json; charset=utf-8"                   (pact: a string)
#   ["application/json; charset=utf-8"]                 (contract: a list of lines)
janus explain samples/order-service/pacts/web-app-order-service.json --executed pact-values.json  # exit 0
janus explain contract.janus.json --executed janus-values.json                                    # exit 1
```

### Why the fix is not just "use `match:header-value`"

`match:header-value` exists only for the legacy compiler, and plan-grammar spec §4.4 keeps it there:
it is v1–v4's specified default written down, not a shape. Giving the shape language an unnamespaced
media-type operator would put HTTP knowledge in the core vocabulary, which shape-language spec §3.5
forbids outright ("the kernel knows nothing about HTTP") and which kernel-boundary-review.md exists to
catch. The rule the RFC and ADR 0007 both want is that a native Janus contract can say what a v1–v4
pact said by default — here, "this media type, with at least these parameters" — as an explicit,
reviewable shape.

### Options

- **A. `http:media-type`, contributed by the HTTP component** (shape spec §3.5; component-interfaces
  spec §7). The same route §3.5 takes for `http:status-class`: a namespaced operator owned by the
  component that knows what a media type is, with the requirement recorded on the interaction.
  Semantics as the author states them: the type and subtype must match (case-insensitively); every
  parameter the shape names must be present with that value; parameters it does not name are admitted
  and ignored — which is shape-language §4.3's must-ignore default applied to parameters, so it fits
  the language rather than bending it. Comparability can be declared `exact` (component-interfaces
  §7.5): `admits(P) ⊆ admits(C)` iff the types match and C's named parameters are a subset of P's with
  equal values, which the subsumption checker can decide. Costs: parsing needs a `matcher/apply`
  action rather than a core-only fragment, and the HTTP component, today a transport only, gains a
  matcher interface. Note that `http:status-class` is in the same position: the upgrade already writes
  it into contracts, and no loaded component implements it yet.
- **B. A generic, HTTP-free core operator** — a parameterised-token or "structured string with named
  parameters" comparison general enough to be core. Avoids a component, but no second user of it is in
  sight, and a core operator is forever (shape spec §3.5: unnamespaced names are reserved for the
  specification). Likely to end up HTTP-shaped under a neutral name.
- **C. Keep `equality` and make the upgrade write a looser shape** — e.g. a `regex` anchored on the
  media type. Needs no spec change, but loses the "a named parameter must match" half: a regex that
  admits any charset admits the wrong one too, which is exactly the case that must fail. Also leaves
  native contracts with no way to say it, only upgraded ones.

Differences 2 and 3 (list whitespace, repeated lines) are the same question for list-valued headers
generally (RFC 9110 §5.3 lets a recipient combine repeated field lines with commas), and option A's
component is the natural owner of that too — either the same operator family, or a transport-level
normalisation the HTTP component applies when it builds the headers slot. Which of the two is a
component-interfaces question: does the transport present a header as its lines or as its combined
value?

**Suggested for review:** option A, specified in shape-language §3.5 and component-interfaces §7
before any code, with the upgrade then writing `http:media-type` for any `Content-Type` (and
`Accept`-like) header without a matching rule, and dropping this `rule-narrowed` finding for them.

### Pointers

- `engine/kernel/src/plan/interpret.rs` — `header_values_match`, `mime_parts`: the v1–v4 semantics.
- `engine/kernel/src/plan/legacy.rs` — `compile_headers`: where the legacy plan uses it.
- `engine/kernel/src/upgrade.rs` — `headers_slot`: the `rule-narrowed` finding; `status_slot`: the
  `http:status-class` precedent.
- Shape-language spec §3.5 (component operators), §4.3 (must-ignore); component-interfaces spec §7
  (matcher interface), §7.5 (comparability); plan-grammar spec §4.4 (legacy-only actions);
  contract-file spec §8.4 (findings).

## 2. Does an array matcher admit the empty array by default?

**Found:** 2026-09-18, fixing the upgrade of cascaded `min`/`max` rules (commit `e34d686`).
**Status:** open by choice — the prototype needs only to be consistent. Resolve it when a real test
framework is built from the prototype (task 9.4).

### The question

When a consumer author writes "an array whose elements are like this one" and says nothing about
length, is `[]` a match? The Pact community has two long-standing answers:

- **Implicitly bounded.** An `eachLike` promises at least one element — the example the author wrote
  down is evidence the list is non-empty, and an empty list is a case the consumer never showed it
  handles.
- **Explicit bounds only.** A type matcher on an array constrains its elements and nothing else; a
  length constraint is a separate thing the author states (`min: 1`) or does not.

Pact-JVM answers both ways, with a different DSL operator for each. The disagreement is about what the
DSL means, not about matching, which is why a prototype cannot settle it by testing.

### Why it matters more in Janus

In Pact, the default decides only what *matches*. In Janus it also decides what is *exercised*:
`each-like`'s cardinality is a variant dimension (shape-language §5.5, §6.4), and `min: 0` puts the
empty array into the variant space as a case the consumer must demonstrate it handles. So "admits
`[]` by default" is also "every array the author writes generates an empty-list variant by default" —
more variants, and more consumer tests, for every list in every contract. The two schools' positions
land differently here than they did in Pact: the implicit bound keeps the variant space small; explicit
bounds make the empty case something the author opts into, visibly.

### Where the prototype stands

| Path | A type rule on an array, no length stated | Admits `[]`? |
|---|---|---|
| Native Janus shape | `each-like`, `min` defaults to 1 (shape-language §4.3, §5.5); `min: 0` is explicit | no |
| v1–v4 pact, verified where it stands (design 3.5 plan) | `match:type` on the array, then each element | yes |
| Same pact, upgraded (contract-file §8.2) | `each-like` with the default `min: 1` | no |
| A rule that only *cascades* to a nested array, upgraded | `each-like` with `min: 0` (commit `e34d686`) | yes |

Native Janus is consistent with itself. The seam is the upgrade: a bare `type` on an array in a v1–v4
pact accepts `[]` in the pact's own plan and rejects it in the contract the pact upgrades into, and the
upgrade raises no finding for it (`engine/kernel/src/upgrade.rs`, `cardinality`, whose comment chose
`min: 1` as "the same default `eachLike` has always had"). The contract is stricter, so this produces
false failures rather than false passes, but it breaks plan-grammar §4.4's two-path agreement for an
empty array. The cascaded row is not part of the question: no author wrote anything about the nested
array, so there is no default to interpret, only v1–v4's behaviour to preserve.

### What resolving it involves

1. **The default of `each-like`'s `min`**, in shape-language §4.3/§5.5 — with the variant-space cost
   above as part of the argument, not an afterthought.
2. **Whether the SDK DSL offers one array operator or two** (sdk-specification): one with a default,
   or Pact-JVM's route of separate operators so an author never relies on a default at all.
3. **How the upgrade maps a bare `type` on an array** (contract-file §8.2): to whatever (1) decides,
   and, if that differs from v1–v4's "any length", with a finding — `rule-narrowed` (`judgement`)
   is the existing code that fits — so the two-path disagreement is reported rather than silent.

Until then: leave `each-like`'s default at 1, and treat the upgrade's silent `min: 1` as the known
inconsistency this entry records.

## 3. The WASM embedding cannot host a consumer test's mock server

**Found:** 2026-09-19, starting plan task 6.2. **Status:** open — the TypeScript SDK ships the
subprocess embedding only, behind a frame-pipe interface a WASM embedding can implement later.

ADR 0003 makes the jco-transpiled WASM component Node's *primary* embedding, with the subprocess as
fallback. For a consumer test that cannot work as things stand, for two independent reasons, both
already written down but never put side by side:

- **Sockets.** A consumer test needs a mock HTTP server. Spike 1.2 scoped the WASM guest to no
  sockets at all ("1.3 remains motivated by native-transport needs (real sockets for mock servers)",
  finding 8), and ADR 0013 says a WASM-guest engine hosts in-tree components only — but the in-tree
  HTTP transport is itself the thing that needs ambient capability.
- **Threads.** The exchange loop that answers the mock's requests runs on a thread of its own
  (`engine/kernel/src/protocol/exchange.rs`, whose header already names this gap), and a plain
  `wasm32-wasip2` guest has nothing to schedule a spawned thread onto.

So for Node — the RFC's own example language — "WASM preferred" does not hold for the one thing an SDK
does most. ADR 0003's tripwire ("If real projects routinely need a third-party component …") is about
components; this is a stronger case, about the built-in transport.

**Options:** (a) accept the subprocess as Node's primary for consumer tests and amend ADR 0003's row;
(b) run the engine component in a Node worker thread with WASI sockets through jco's preview2-shim,
and rework the exchange loop into a single-threaded poll the host drives — a kernel change, and
unproven; (c) host the mock transport on the SDK side of the pipe — ADR 0013's rejected "trampoline"
alternative, which puts transport code in every SDK.

## 4. A consumer's test verdict has no way into the engine

**Found:** 2026-09-19, plan task 6.2. **Status:** open — the TypeScript SDK withholds the contract
itself (behavioural spec `finalise`).

The engine verifies each *exchange*: the request the consumer sent matched the armed variant. It
cannot know whether the consumer then *handled* the response — that is the closure's verdict, and it
lives only in the SDK. So for task 4.6's careless client, which crashes whenever `shippedAt` is absent,
every exchange verifies and `finalise` returns a contract claiming the consumer exercised the
`shippedAt=absent` variants — the ones its own test failed on. Writing it would break the honesty rule
(contract spec §2.2) from the other side.

The TypeScript SDK therefore refuses to write the contract when any `execute` in the suite failed, and
says so. That is correct, but it is the one verdict an SDK holds that the engine does not, so every
SDK must reimplement it identically, and a host that forgets gets a dishonest contract with no error.

**Options:** a `consumer-session/report-variant { session, handle, variant, status, reason? }`
operation (or a `failed` member on the next `serve-variant`) so the engine records the closure's
verdict and withholds the contract itself — protocol-additive, and it moves the decision back where
B1 wants it.

## 5. Engine-side failures are only attributable at the end of a suite

**Found:** 2026-09-19, plan task 6.2. **Status:** open.

`consumer-session` has no per-interaction result before `finalise`, and no events. With one session
per suite (SDK spec §4's SHOULD, and the only arrangement that writes one contract for a suite without
the SDK merging contracts), a request the mock could not match — or a variant the closure never
exercised — surfaces in the suite's `afterAll`, not in the test that caused it. The mock's 500 usually
makes the test fail anyway, but a client that swallows errors, or a closure that never calls the mock
for one variant, fails the suite rather than the test.

**Options:** results per interaction on demand (`consumer-session/results { session, handle }`), or
consumer-session events on the existing poll stream (engine-protocol spec §9), either of which lets
`execute` fail the test that caused the failure.

## 6. An SDK writes contract bytes it can only reconstruct

**Found:** 2026-09-19, plan task 6.2. **Status:** open — documented in the TypeScript SDK's STYLE.md.

`finalise` returns the contract as a document inside a JSON frame, and persistence is the host's
business (engine-protocol spec §8.2) — so the SDK parses it with the frame and re-serialises it, and
the canonical bytes (contract spec §2.4, ADR 0018) are whatever the SDK's JSON writer makes of the
parsed value. For TypeScript that matches the engine's writer except where JavaScript cannot help it:
integer-like member names (`"10"`, `"9"` in a shape's `members`) are enumerated first in ascending
numeric order, whatever order the engine wrote, and a few number spellings differ. Two SDKs recording
the same content can therefore write different bytes, which is what ADR 0011's determinism argument
(broker dedup, clean diffs) exists to prevent — and SDK conformance deliberately compares content, not
bytes (ADR 0017), so the suite would not notice.

**Options:** carry the contract as canonical text (a tagged `encoded: "text"` member, protocol §2.5)
so the SDK writes the engine's bytes verbatim; or make the engine write the file itself for hosts
that ask it to (it has a filesystem in the subprocess embedding, not in WASM).

## 7. "This header must not be sent" is unreachable from the DSL

**Found:** 2026-09-20, plan task 6.5's regeneration trial. **Status:** open.

A `forbidden` helper written as a header or query value builds a document the engine refuses, and the
document the author meant has no spelling. Behavioural spec `request` compiles a header or query map
to an `object` node whose members are lists, so a shape helper written as a value becomes
`each-like { items: <shape>, min: 1, max: 1 }` — bounded at one precisely so it contributes no
cardinality dimension. But shape spec §5.1 says a node admitting absence may appear only in a slot,
and an `each-like`'s `items` is not one. So:

```
headers: { "x-trace": forbidden() }        →  each-like { items: forbidden, min: 1, max: 1 }
```

which the engine rejects at `add-interaction`, pointing at `/parts/request/headers/members/x-trace/items`
— a pointer into a node the author never wrote. Meanwhile the shape the author meant is *already
legal*: a headers map is an `object` node, its members are slots, and
`{ "x-trace": { "shape": "forbidden" } }` is accepted by the engine today. Only the DSL cannot say it.

Both SDKs have the rule and both have the gap; neither is wrong, because the behavioural
specification does not say what `forbidden` composed with `request`'s name-to-list rule means. The
TypeScript regenerating agent found it by reading the two entries against each other, and it is
recorded here rather than fixed because the fix is a change to `request`'s semantics, which is a
design decision and not a task-6.5 one.

**Reproduce:** `headers: { "x-trace": forbidden() }` in either SDK, or send the wrapped document to
`consumer-session/add-interaction` directly; compare with the bare `forbidden` member, which is
accepted.

**Options:** (a) `request`'s name-to-list rule gains an exception — a `forbidden` helper in a header
or query map becomes that member's shape directly, unwrapped. The rule's stated reason for wrapping
(an unbounded `each-like` would contribute a cardinality dimension) does not apply to a node admitting
exactly one thing, and this is the only option that makes the useful assertion expressible. (b)
`forbidden`'s entry states that a header or query map is not a position it may be written in, and the
SDK refuses it at the call — but that refuses something the shape language permits. (c) Record it as
deliberately undecided, per `conformance/README.md` §7. A conformance case should follow whichever is
chosen; there is none today, which is why both SDKs could ship the gap without the suite noticing.
