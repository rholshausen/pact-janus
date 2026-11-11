# 0082 — Compare a list-valued header as its elements, when the contract says it is a list

- **Status**: accepted
- **Date**: 2026-11-11
- **Plan tasks**: S3.3 (finding 1)
- **Evidence**: RFC 9110 §5.2, §5.3 and §5.6.1 (field lines, combined field values, lists), RFC 9113
  §8.2.3 and RFC 9114 §4.2.1 (HTTP/2 and HTTP/3 cookie crumbs); pact-reference's and pact-jvm's
  header history: pact-jvm #851, #997, #1159, #1288, #1852, #1893 and #1862, pact-reference #259, #300,
  #353 and pact-js #1058, which moved them from splitting every header on commas to splitting none to
  splitting twelve names (`pact_models::headers::MULTI_VALUE_HEADERS`), and pact-jvm's fix of
  2026-11-03, which matches a rule against the whole value when the split one fails; S1.1's ledger
  entry for `compatibility-suite/v2/01-supports-matching-rules-for-the-response-headers-positive-case`;
  [ADR 0071](0071-compare-a-content-type-as-a-media-type-with-the-http-components-http-media-type.md)
  and [ADR 0057](0057-the-http-component-runs-http-status-class-as-an-in-tree-matcher.md), whose
  mechanism this reuses

## Context

HTTP sends a header as one or more field lines. Where a field is defined as a list (`Accept`, `Vary`,
`Cache-Control`), RFC 9110 §5.3 makes its field lines one value, joined with commas, and the framing
means nothing. `Accept: a, b`, `Accept: a,b` and two lines `Accept: a` and `Accept: b` are the same
header. Where a field is not a list, a comma is just a character (`Date`, a JSON value, `Set-Cookie`,
which is never combined), and two field lines are an error or two separate values.

The HTTP transport presents a header as its field lines, unsplit: `{"accept": ["a, b"]}` and
`{"accept": ["a", "b"]}` for those two framings (shape spec §3.6). A native shape compares the lines
exactly, so a consumer that declares `accept: "a, b"` fails a client that sends `a,b`, or the same
elements on two lines. Today's Pact passes both.

Pact itself has not settled this. pact-jvm and pact-reference split every header on commas until JSON
headers broke, then exempted known single-value names, then (2025–2026) split only twelve names known to
be lists. Last week pact-jvm found that splitting `Accept` broke a regex written against its whole
value, and now tries each element, then the whole value. Each fix was right for the report in front of
it, and wrong for an earlier one. The cause is that the matcher decides, from the header's name, what
the author meant.

Three more problems come from the same gap:

- **The legacy compiler joins a repeated header with `", "` and applies its rule to the joined text.**
  Today's verifier applies the rule to each value. S1.1's ledger carries the disagreement: `X-TEST: 1`
  and `X-TEST: 1000` against `\d{1,4}`.
- **`janus upgrade` writes a header that `pact_models` split as one entry per element.** A pact's
  `Accept: application/json, text/plain` reads as two values, and a reply carrying them on one line
  fails the contract, which the pact passes.
- **`janus upgrade` writes a header with a comma as `equality`, and reports the narrowing.** v1–v4 ignore
  whitespace around commas in any header.

## Decision

1. **The transport presents a header as its field lines, in every HTTP version, and never splits
   one.** HTTP/1.1 field lines arrive as sent. HTTP/2 and HTTP/3 carry the same field lines, except
   that a client may split `Cookie` into crumbs. The transport joins them with `"; "` into one line,
   as RFC 9113 §8.2.3 and RFC 9114 §4.2.1 require before a value leaves the protocol. `cookie` then
   reads the same in every version.
2. **A header is a list when its shape says so: `http:list`, from the built-in HTTP component.** It is
   a component operator, run as `http:media-type` is (ADR 0071), and written as the header's member,
   not inside the list of its lines. Its member `elements` is the elements expected, in order. (Not
   `items`, which the shape schema gives every node as the nested shape of an `each-like`.) It admits
   a header whose field lines, joined with commas, split into exactly those elements. The split
   happens at each comma outside a quoted string, whitespace around each element is dropped, and
   empty elements are ignored (RFC 9110 §5.5 and §5.6.1). Elements compare exactly. So `a, b`, `a,b`,
   `a, , b` and two lines `a` and `b` all match `elements: ["a", "b"]`; `b, a` does not, since the order
   of a list's elements is significant in general (RFC 9110 §5.3). An absent header fails the node.
   Its example is the elements joined with `", "`, one field line, which is what a mock serves.
3. **Nothing is split because of its name.** No registry of list-valued fields decides how a contract
   matches: the contract says so, in its own document. A header with no `http:list` is compared as
   its field lines, exactly as before.
4. **The native DSL spells it `headerList(...)`** (`Shapes.headerList(..)` in Java), valid as a header's
   value. A list of strings keeps meaning several field lines, exactly, which `Set-Cookie` needs. The
   helper is the one addition to the surface put to review in S3.2, and the review note asks about it.
5. **`janus upgrade` writes `http:list` where v1–v4 compare a header with no rule as a list:** when
   `pact_models` read several values, or a value with a comma. Its `elements` are the pact's values joined
   and split as decision 2 splits them. The `rule-narrowed` judgement stays only where a header that
   is not `Content-Type` has a value with parameters (`;`). v1–v4 compare such a value's parameters
   as a set, extra ones allowed, and an element of `http:list` compares exactly. *Amended 2026-11-11*
   ([ADR 0083](0083-compare-accept-and-content-type-as-media-types-and-every-other-headers-parameters-as-text.md)):
   today's verifier reads parameters only in `Accept` and `Content-Type`. An `Accept` of one media
   type is `http:media-type`, and the judgement is left only on an `Accept` of several elements.
6. **A header's rule applies to each of its values, as today's verifier applies it.** The legacy
   compiler compiles a header with a rule as a `flow:for-each` over the reply's values, each split as
   today's verifier splits it (`pact_models::headers::parse_header`). Its default comparison,
   `legacy:header-value`, reads a header with several values as their join. `janus upgrade` writes a
   header with a rule as an `each-like` of the rule's shape, with a minimum of one.

7. **A contract writes a component operator's members by name.** Each SDK builds `http:list` in the
   order its language's bindings write members, and the engine wrote a component operator's node as
   it arrived (contract spec §2.4), so the two SDKs' contracts differed by a byte. A component reads
   its members by name, and nothing is decided by their order (ADR 0037), so the engine now sorts a
   component node's member names, as it sorts a core node's, and writes each value as it arrived.
   An existing contract with `http:media-type` or `http:status-class` is written with that node's
   members in another order the next time it is written, and reads as before.

## Alternatives considered

- **The transport splits the fields RFC 9110 defines as lists, and presents their elements.** No
  author writes anything, and a list header always reads as its elements. But it is the registry Pact
  has rewritten four times. A regex on `Accept`'s whole value, the report pact-jvm fixed last week,
  would see elements and fail. And any change to the list would change how existing contracts match.
- **A list of strings in the DSL means a list header.** It is what an author writes for `Accept`. But
  `Set-Cookie`'s values hold commas in their dates and are never combined, so the SDK would need the
  registry decision 3 refuses to know which headers a list may describe.
- **Element shapes inside `http:list`, such as a regex per element.** A component operator is a leaf
  the kernel does not look inside (shape spec §3.5), so this would change the shape language for one
  operator. Exact elements cover what v1–v4 compare with no rule. A rule on a list header still
  compiles, as an `each-like` over its field lines.
- **Compare elements as media types in `Accept`.** v1–v4 do (`PARAMETERISED_HEADERS`). It needs a
  second comparison mode on the operator, for one header whose parameters are rarely written in a
  pact. The judgement says where it would differ.
- **Ignore element order.** HTTP gives some lists an order (`Content-Encoding`, `Via`), and v1–v4
  compare by index.

## Consequences

Easier: a consumer that declares `headerList("application/json", "text/plain")` passes however its
client frames them, and a provider's `Vary: Accept,Origin` passes a contract that says `Vary: Accept,
Origin`. A contract upgraded from a pact with a list header verifies as the pact does. S1.1's
repeated-header disagreement goes.

Harder: a header the author does not declare a list is compared as its field lines, so framing still
matters there, as it does for a custom header in today's Pact. An upgraded contract with a list
header requires the `http` component, which every native embedding has.

Committed to: `http:list` and its `elements` member, in contracts that outlive this engine, and the
splitting rule in decision 2.

Differences from today, which no fixture shows: v1–v4 drop whitespace around a comma inside a quoted
string, and `http:list` keeps a quoted string as written. v1–v4 count an empty element, and
`http:list` ignores one.

Tripwire: if consumers routinely need a shape on a list header's elements (a pattern per media type,
say), decision 2's exact elements are revisited, with element shapes as a shape-language change.
