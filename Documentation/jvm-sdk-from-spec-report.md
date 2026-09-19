# Plan task 6.3: the JVM SDK written from the specification alone

**Date:** 2026-09-19. **Subject:** `sdks/jvm/sdk`, Java 17, JUnit Jupiter, subprocess embedding.
**Claim under test:** SDK spec §3.3. A `semantics` paragraph passes if "a competent engineer,
fluent in the target language but who has never read another language's idiomatic-layer source, can
implement the primitive from it alone and pass every conformance-suite case its `conformance` list
names."

## 0. Provenance

**I did not read the TypeScript SDK.** I did not open, list, grep or diff anything under
`sdks/typescript/` or `cli/tests/janus-engine-node/`, and I did not read their git history. The inputs
were:

- the SDK specification (`spec.md`, `behavioural-spec.json` and its examples);
- the engine-protocol, shape-language, variant-semantics and contract-file specs and schemas;
- ADR 0003, ADR 0017 and Phase 9 findings §3–§6;
- the generated JVM bindings;
- `sdks/README.md`.

Phase 9 findings 4–6 were written during task 6.2 and describe TypeScript-SDK decisions in prose.
Finding 4 restates what the canonical `finalise` entry already requires. I noted finding 6, the
byte-reconstruction problem, and designed around it (§3.6). Neither told me anything about how the
TypeScript code is structured.

Where the specification ran out, I probed the real `janus-engine` as a black box, with a throwaway
Python frame client in a scratch directory (not committed). Everything I learned that way is marked
as an engine observation below, not as something the spec said.

## 1. What was built

| | |
|---|---|
| Idiomatic layer | `sdks/jvm/sdk/src/main/java/io/pact/janus/sdk/**`: 24 files, about 2,200 lines including Javadoc. Every one of the 23 primitives in `behavioural-spec.json` is implemented. |
| Embedding | `SubprocessEmbedding` (Content-Length framing, stdin EOF as shutdown, kill escalation) behind `Embedding`/`FramePipe`. `FramePipe.call(byte[]) → byte[]` is the WASM call pipe's shape, so a Chicory embedding would slot in there. Chicory was not attempted, per the owner's decision. |
| JUnit integration | `JanusExtension` (`AfterAllCallback` → `finalise`). Offers `execute` inside a `@Test`, and a `@TestFactory` form with one dynamic test per variant. |
| Tests | 57 in the `:sdk` project, all passing. 5 more are reported as skipped: they are classes that only run inside the launcher harness, where their outcomes are asserted. Tests are organised by SDK spec §7: `TranslationTest` (27), `SessionLifecycleTest` (14), `VariantIterationTest` (5), and `EndToEndTest` (10) plus `OrderConsumerTest` (1) against the real engine. Every one of the 52 conformance ids in the behavioural spec is named by at least one test's display name. `./gradlew build` from `sdks/jvm` passes: 60 tests run, 0 failures, 5 skipped. That includes the 3 existing bindings tests. |
| Build | `:sdk:buildEngine` runs `cargo build -p pact_janus_cli --bin janus-engine` before every test run and sets `JANUS_ENGINE`, so the tests never run against a stale engine. |
| Style guide | `sdks/jvm/STYLE.md`, every section of the skeleton filled, with 8 deviations. |

**The RFC example**, written as naturally as Java allows (`OrderConsumerTest`), runs all 8 variants
the engine selects against a real `java.net.http` client and writes
`build/contracts/web-app-orders-api.janus.json`. A **careless consumer**, one that dereferences
`shippedAt` without checking, fails with an `ExecuteFailedException`. That exception names exactly
the variants whose assignment has `response.body.shippedAt#presence=absent`, with their
`NullPointerException`s. `finalise` then throws `ContractWithheldException` and writes no file. It
does the same in the `@TestFactory` form: the three `shippedAt=absent` variants fail as their own
tests, the other five pass, and the class fails for the withheld contract.

## 2. Primitive by primitive: where `semantics` was not enough

**Count: 17 of 23 primitives were implemented straight from their `semantics` (plus the designs they
cite). 6 needed at least one guess:** `janus`, `given`, `request`, `literal`, `execute`, `finalise`.

The 17 straight ones: `interaction`, `response`, `json`, `integer`, `number`, `decimal`, `string`,
`boolean`, `datetime`, `date`, `time`, `regex`, `any-of`, `one-of`, `optional`, `nullable`, `each-like`.
`response` counts as straight because every gap it inherits from `request` is counted there. Three
Java-specific spellings among them (`bool`, `Cardinality`, `anyOf(null)`) are style, not ambiguity.
STYLE.md records them.

A caveat applies to every "would a conformance test catch it?" answer below. **The conformance suite
(task 6.4) does not exist.** The `conformance` lists are scenario names with no scenarios, so I had to
invent the scenario behind every one of the 52 ids. No test anywhere can yet say whether my reading of
an id matches anyone else's. Each answer below is about the scenario the id's name most plausibly
implies.

### 2.1 `literal`: "in the order written", and what counts as a map or a list

> "An object — a plain map written in the DSL … — becomes an 'object' node with one member per key …
> in the order written"

- **Missing:** Java has no ordered map literal. The natural spelling, `Map.of(...)`, discards the
  written order. Worse, its iteration order is salted **per JVM run**, so obeying "iteration order"
  would make the same DSL produce a differently ordered document on every run. `Map.of` also rejects
  `null` values, which makes `{ note: null }` unwritable.
- **Chose:** maps with a defined encounter order (`LinkedHashMap`, `SortedMap`) keep it. Maps without
  one are written in key order. I added `Shapes.map(k, v, …)` as an ordered, null-admitting map
  literal. It is plain data, not a helper. This is a STYLE.md deviation.
- **Also missing:** what "a list" and "a string, number, boolean or null" mean in a typed language.
  Should a `Collection`, an `Object[]` or a `char` count? What about an `Instant`, an enum or a POJO?
  **Chose:** `List` is a list. `String`, `Boolean`, any `Number` (not NaN or infinity) and `null` are
  scalars. Anything else is refused with `IllegalArgumentException` ("Nothing else is inferred").
- **Would a conformance test catch a different choice?** Not unless it checked member order across two
  JVM runs, and ADR 0017 compares content, not order. A different choice would, however, change
  contract bytes run to run. That would be caught by nobody except a broker that stopped
  deduplicating. (In practice the engine re-sorts members anyway: §3.5.)
- **Language-neutral rule the spec could state:** "members in the author's order where the language
  preserves one, otherwise in a deterministic order the style guide names."

### 2.2 `given`: "no key … reordered"

> "'params' is passed through as written — no key is renamed, reordered or coerced."

This is the same gap as §2.1, in a place that explicitly forbids reordering. For `Map.of` params
there is no written order to keep, so the SDK applies the same key-order rule, which technically
contradicts the letter of the text. Also unstated: `given(name, null)` versus `given(name, Map.of())`.
I chose `null` to mean "not given" (no `params` member) and an empty map to be passed through as `{}`.
`session.given.params-omitted-when-absent` would catch the first only if its scenario passes a null.

### 2.3 `request`: the name-to-list rule has holes, and one real defect

> "a string value becomes an 'equality' node over the one-element list [value]; a list of strings
> becomes an 'equality' node over that list; a shape helper describes each value and becomes an
> 'each-like' node whose 'items' is that shape."

1. **Other value types are unspecified.** A Java author writes `.header("X-Count", 3)` or
   `.query("page", 2)` constantly. **Chose:** refuse anything but those three forms, because §2.1
   forbids coercion. A different SDK stringifying `3` would pass every conformance id listed, since
   none names this case.
2. **Case collisions are unspecified.** "Header names are lower-cased" means `Accept` and `accept`
   become one slot. Which one wins, or are they merged? **Chose:** refuse the second declaration.
   Not covered by any listed id.
3. **Calling `request` twice is unspecified.** Replace, or merge per slot? **Chose:** replace. Not
   covered by any listed id.
4. **A defect, confirmed against the engine.** The `each-like` produced for a helper-valued header
   or query parameter has no bounds, so it defaults to `min` 1, `max` unbounded. The engine
   therefore derives a **request-side cardinality dimension** with points `min` and `min+1`. The
   consumer is then required to send the header **twice** in a second variant, or the contract is
   withheld. `.header("X-Trace", regex("^[0-9a-f]+$", "abc123"))` is unusable with an ordinary
   client (test `EndToEndTest#helperHeaderValueAddsACardinalityDimension`). The shape spec §3.6
   sketch that this rule cites shows the same construct, on a *response* header, where it would make
   the provider-side variant space grow instead. No `session.request.*` id would catch this, because
   the document is exactly what the spec says to emit. The spec should either fix the bounds (for
   example `min: 1, max: 1` unless the author gives a `Cardinality`, which collapses the dimension
   per shape spec §6.1) or say that this is intended.

Two further contradictions are between the worked example and the canonical entry; the canonical
entry wins by §3.1. `order-example-mapping.md` §3.1 says the `interaction-invalid` problems are
"rewritten from the interaction-spec path to the DSL call site". The canonical entry says "carried
verbatim". The SDK passes them verbatim.

### 2.4 `janus`: the version-mismatch error that does not exist

> "A hello that fails, or agrees any protocol version other than 1, fails that first 'execute' with
> the engine's error"

When the engine *agrees* a version other than 1, there is no engine error to fail with. **Chose:**
`JanusEmbeddingException`, which is a STYLE.md deviation. Also unstated: whether the configured object
is usable after `finalise`. The text says "one engine … belong[s] to one configured object".
**Chose:** yes; the next `execute` starts a fresh engine and session. `session.janus.*` ids would not
catch either choice.

### 2.5 `execute`: machinery failures during the loop, and one test versus many

- **Unstated:** what happens when `serve-variant` itself fails partway through the loop, for example
  if the engine died. Is that a variant failure, so the remaining variants still run, or does
  `execute` abort? **Chose:** it is that variant's failure, and the loop continues (against a dead
  engine every remaining variant then fails fast), so the "every variant runs" guarantee still holds.
  `session.execute.every-variant-runs-after-a-failure` is phrased around the closure and would
  probably not exercise this.
- **Tension with the style skeleton.** The canonical entry says `execute` "then fails once, naming
  every failed variant". The style skeleton says "each selected variant SHOULD appear as its own named
  test result, not as one aggregate pass/fail". In Java one call cannot be both. **Chose:** two entry
  points over one implementation: `execute` (fails once) and `JanusExtension.variants` (a
  `@TestFactory`, where each variant fails as its own test). This is a STYLE.md deviation. The spec
  should say which is normative, or that both are allowed.
- **Wrong citation.** `execute` says the variant descriptor fields are defined in "variant semantics
  spec §5". §5 is "The provider side". The fields are defined in §2.2 (id, label) and §3.9 (the
  selection document: origin, assignment). This was easy to resolve, but it is exactly the kind of
  pointer a blind implementer follows first.

### 2.6 `finalise`: which failures withhold, and where the file goes

- **"when any 'execute' in the session failed"**: does an `execute` that failed *before any variant
  ran* count? For example, one the engine rejected with `interaction-invalid` while the session stayed
  open. The engine's contract then simply lacks that interaction, and the whole suite otherwise
  verifies. **Chose:** yes, it counts, and the contract is withheld. A suite with a rejected
  interaction should not publish a contract silently missing it. A different SDK could reasonably
  choose otherwise. `session.finalise.failed-test-withholds-contract` would catch the difference only
  if its scenario includes a rejected interaction, which its name does not suggest.
- **The filename's citation does not say it.** "'<contract directory>/<consumer>-<provider>.janus.json'
  (contract spec §2.3's filename convention)": §2.3 says only `*.janus.json`. The contract spec's
  own exploded-projection example (§2.6) uses a double hyphen (`orders-ui--orders-api.janus/`). I used
  the behavioural spec's single hyphen. A name containing `-` makes the single-hyphen form ambiguous,
  and a name containing `/` escapes the directory. Neither is addressed.
- **"compact UTF-8 JSON … members in the order the engine returned them"** was clear. It was also
  achievable exactly, which is worth recording (§3.6).
- **Naming inconsistency across designs.** The protocol (§8.2) and schema call the finalise result
  member `contract`. Contract spec §2.6 says the contract "crosses the boundary as the `pact` member
  of `consumer-session/finalise`". `order-example-mapping.md` §4 also writes `{ results, pact? }`.
  The schema wins, but a blind implementer has to notice that.

### 2.7 Worked examples that contradict the canonical document

SDK spec §3.1 says the canonical document wins, and it did. Still, every one of these cost a
deliberate check, and a reader who trusts the example gets it wrong:

| Example text | Canonical entry |
|---|---|
| `order-example-mapping.md`: `execute` produces `consumer-session/finalise` and "finalise ends the session" | `finalise` is its own primitive, run by the test-framework integration |
| `order-example-mapping.md`: `one-of` is "a compile-time error, not a runtime one" for duplicate or missing discriminators, and the DSL "derives" the discriminator node | the SDK does **not** check, and the engine rejects |
| `order-example-mapping.md`: `datetime` may infer `format` if the style guide names the rule | no inference, ever |
| `order-example-mapping.md`: `each-like` conformance `shape.each-like.min-default-1` | `shape.each-like.bounds-only-when-given`: the SDK must *not* write the default |
| shape-language `order-payload.md` §2 (cited by several entries as the DSL-to-shape authority): `"default": "card"` on the one-of, `"format": "yyyy-MM-dd'T'HH:mm:ssX"` on `shippedAt` | no `default`; no format unless given |
| the protocol transcript `consumer-http-session.md`: `start-transport` after `variants`, and interaction parts at the top level (`"request": {...}`) instead of under `parts` | `add-interaction` → `start-transport` → `variants`; `parts.request` |

## 3. Protocol and engine: what surprised me or got in the way

Ordered by how much each would hurt a real JVM user.

1. **The mock never answers the JDK's default `HttpClient`.** `HttpClient.newHttpClient()` offers an
   HTTP/2 cleartext upgrade (`Connection: Upgrade, HTTP2-Settings`). The engine's mock
   (`server: tiny-http (Rust)`) never responds, and a client without a timeout hangs forever. My first
   end-to-end run hung for ten minutes. **Worse, the engine records that exchange as verified, and
   `finalise` returns a contract**, although the consumer never received a response
   (`EndToEndTest#defaultJdkHttpClientIsNeverAnswered`). HTTP/1.1 works. The test clients pin
   `HttpClient.Version.HTTP_1_1`, and STYLE.md tells users to do the same. This is an engine defect
   in the HTTP transport. The "verified without a response" half looks like it matters beyond Java.
2. **Declared response headers are not served, and JSON bodies are not labelled.** A response declaring
   `Content-Type: application/json` and `X-Served: yes` is served with only `Server`, `Date` and
   `Content-Length` (`EndToEndTest#declaredResponseHeadersAreNotServed`). This contradicts the `json`
   entry's premise ("the HTTP transport … labels it application/json"). A client that dispatches on
   `Content-Type`, which Spring's `RestTemplate` and Feign both do, would fail against the mock.
3. **`engine/shutdown` is answered `operation-unsupported`**, although protocol spec §6 says the
   engine MUST implement it. The `finalise` entry only asks for stdin EOF, which works, so the SDK
   never sends `shutdown`. A host following §6 literally would get an error.
4. **No per-exchange verdict before `finalise`** (Phase 9 finding 5, confirmed). In both FINDING
   tests the closure passes and only `finalise`, in the class's `afterAll`, reports the failure. In
   the `@TestFactory` form that means a variant can show green while its exchange failed.
5. **The contract's member order is alphabetical, not contract spec §2.4's.** The engine returns
   `$format, consumer, interactions, provider` and, inside interactions,
   `description, parts, selection, states, transport`. Its frame writer evidently sorts object
   members. §2.4 requires `$format, consumer, provider, interactions` and
   `description, transport, states, parts, …`. The SDK is forbidden to reorder, so every contract any
   SDK writes is non-canonical. It also means the `literal` rule's careful "in the order written"
   (§2.1) never survives into the contract.
6. **Writing the engine's bytes verbatim was possible, on this pipe.** Phase 9 finding 6 says an SDK
   can only reconstruct the contract bytes. On the JVM it does not have to. Jackson's streaming
   parser reports byte offsets, so `EngineClient.okMemberBytes` slices `ok.contract` out of the
   response frame exactly as the engine wrote it, and the file is those bytes plus one LF.
   `SessionLifecycleTest#writesContractBytesUnaltered` proves this with out-of-order members,
   integer-like names, `1.0`, `1e5`, `-0.0`, `\/` and escaped and raw non-ASCII. This holds only
   because stdio frames are JSON. A negotiated binary encoding would bring finding 6 back.
7. **Bindings friction.** None of it blocked the work, but all of it is hand-written glue a
   regenerating agent must also get right:
   - The generated `Shape` has `@JsonInclude(NON_NULL)`, so `equality` over `null`, which "MUST" carry
     its example (shape spec §3.3), silently loses `"example": null`. The workaround is to put it in
     `additionalProperties`.
   - The protocol bindings type every embedded document as `Map<String, Object>`
     (`AddInteraction.interaction`, `RequestFrame.body`, `ResponseFrame.ok`, `FinaliseResult.contract`),
     and `ShapePart`'s values are maps rather than `Shape`. So the typed `InteractionSpec` and `Shape`
     bindings must be converted to maps with Jackson before they can be sent.
   - `VariantDescriptor` binds only `id`. The richer `variant.v1.Variant` exists but is not what the
     protocol result deserialises into.
8. **A second `start-transport` starts a second transport.** Both succeed with different ports. The
   SDK keeps one per session, as the entry says, but nothing in the protocol stops a host leaking
   transports within a session.
9. **Environment.** Nothing blocked the work. JDK 17, cargo and Gradle 9.7.1 were all present. The
   engine builds in about a second when unchanged. The one lost stretch was the HTTP/2 hang in item 1.

## 4. Verdict on SDK spec §3.3

**Partly true, and only half testable today.**

- **Where it holds:** the shape primitives. Seventeen of the 23 entries, with the shape spec they cite,
  were enough to implement without guessing. The translation tests pass against a document I derived
  from the prose, and the real engine accepts every node and derives exactly the dimensions the
  shape spec predicts. The discipline of citing other designs by section works here, because the
  shape spec is precise. The one miss in this group is the wrong §5 citation, which was cheap to
  resolve. The same goes for the protocol-facing half of `execute` and `finalise`: create → add → start
  → variants → serve*/closure → finalise, one session per suite, verbatim errors. I implemented these
  without needing anyone's source, and the engine agreed on the first run.
- **Where it does not:** the lifecycle and translation edges. Six entries needed at least one guess:
  2 guesses each for `janus` and `given`; 4 plus a defect for `request`; 2 for `literal`; 2 for
  `execute`; and 2 plus 2 naming problems for `finalise`. The guesses are not the kind "look at the
  TypeScript SDK" would settle safely either; they are unstated decisions. Two SDKs could make them
  differently and both pass every conformance id as named. The worst are the ones that change
  **what gets published**: whether a rejected interaction withholds the contract, and whether a
  helper-valued header is usable at all. Some prose also quietly assumes a language with ordered map
  literals ("in the order written", "no key reordered"). A spec meant to be language-independent
  should state what an implementation does where the language cannot honour that.
- **Why "only half testable":** the bar is "pass every conformance-suite case its `conformance` list
  names", and there are no cases, only names. Every judgement call above passes the scenario I
  invented for its id, because I wrote both. Task 6.4's suite is what would turn §3.3 from a claim
  into a result. Until it exists, this report is the evidence, and it says the format transmits the
  *happy path* reliably and leaves the *edges* to the implementer.

**Recommended changes to the SDK spec** (none made here, per the task's rules):

1. State the rule for maps with no preserved order.
2. Say what `request` does with non-string values and with case-colliding names, and whether a second
   call replaces or merges.
3. Fix or bless the helper-valued header cardinality dimension.
4. Say whether an `execute` rejected before any variant ran withholds the contract.
5. Fix the §5 citation, the `<consumer>-<provider>` citation and the `pact`/`contract` naming.
6. Reconcile `order-example-mapping.md` and `order-payload.md` §2 with the canonical entries, or mark
   them superseded.
7. Say whether per-variant test results (the skeleton's SHOULD) may replace `execute`'s single
   failure.

**Engine issues to file:**

1. The mock hangs on an HTTP/2 upgrade offer, yet the engine counts the exchange verified.
2. Declared response headers are not served.
3. `engine/shutdown` is unimplemented.
4. The contract's member order is not canonical.
