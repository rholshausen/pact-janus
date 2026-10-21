# 0036 — Read text bodies through a text component, and bodies no component claims as bytes

- **Status**: accepted
- **Date**: 2026-10-21
- **Plan tasks**: S1.6 (the build); S1.21 to S1.23 (the body kinds that reuse decision 3); S1.25
  (`janus upgrade`, which this leaves as it was); S6.5 (charsets other than UTF-8)
- **Evidence**: pact-reference's matcher (`pact_matching::match_body`, which picks a body matcher by
  the pact's content type once the two content types agree; `compare_bodies` and its
  `BODY_MATCHERS`; `match_text`; `binary_utils::match_octet_stream`); `pact_models`'
  `ContentType::is_binary` and `body_from_json`; S1.1's first run
  (`compatibility-suite/v1/15-…` and `…/21-…`); component-interfaces spec §5.5, §6.1;
  plan-grammar spec §6

## Context

The engine has one content component, JSON. A v1–v4 pact whose body is anything else fails before
anything is sent. S1.1's first run found two cases. A `text/plain` body is refused as
`component-unavailable`. A binary one is worse: `pact_models` base64-decodes an `image/jpeg` body and
drops its content type, so the legacy layer tries it as JSON and refuses the pact as
`contract-invalid`. Today's verifier passes both.

Today's verifier picks a body matcher by the **pact's** content type: the body's own, else the
`Content-Type` header, else one detected from the body. It first checks that the reply's content type
agrees with it, or that one of them is unknown; a disagreement is a failure. JSON, XML, multipart and
form bodies have matchers of their own. A binary type (`image`, `audio`, `video`, `font`, and anything
`pact_models` does not count as text) is compared as bytes. Everything else is compared as text. Bytes
and text are compared the same way: by equality, or by the rules on `$` when there are any.

So the engine has to be able to hold a body as text, and as bytes. It must do so without the kernel
learning what either is, and in a form S1.21 to S1.23 can reuse for form, XML and multipart.

## Decision

1. **Text is a built-in content component, `text`, declaring `text/*`.** It decodes a slot's octets
   as UTF-8 into a string, and encodes a string into UTF-8 octets labelled with the content type it
   was asked for. A body that is not UTF-8 does not decode, whatever its `charset` says, and a
   document that is not a string does not encode. It contributes no plan fragments and detects
   nothing. Every embedding registers it after JSON. Since the registry asks components in order, a
   component a project declares for a `text/` type comes first, as S1.22's XML component will for
   `text/xml`. Claiming all of `text/*` must not satisfy a slot that needs another component, so its
   `compile` refuses a shape that admits no string, and the engine asks the in-tree component that
   handles a slot to compile it when the interaction arrives, as it asks a declared one
   (component-interfaces spec §6.3). A `text/csv` body of rows with no CSV component declared is still
   refused as `component-unavailable`, naming `content/text/csv`; a `text/csv` body that is one
   string is served as text.
2. **A content slot whose media type no component claims is its octets: a bytes document.** The
   document model has bytes (component-interfaces spec §6.1), and a body nobody can read is still a
   body. Until now such a slot was matched as absent. A component that claims the type and fails to
   decode it still leaves the slot absent, as before: that is a body that is not what it says.
3. **The legacy layer reads a v1–v4 body the way today's verifier picks its matcher**, by the pact's
   content type. JSON is a document, as before. XML, multipart and form bodies are still refused as
   `component-unavailable`, until S1.21 to S1.23 add them. A binary type is bytes. Any other type is
   text: a string, or bytes when it is not UTF-8. The legacy compiler compiles a text or bytes body as
   one value at `$`: the rules on `$` if there are any, equality otherwise. This is `match_text` and
   `match_octet_stream` in plan form.
4. **A provider's reply body is decoded as the pact's content type, and the plan checks that the
   reply's type agrees with it**, as today's verifier checks before it compares a body. The
   verification declares the pact's type for the response body, as a contract declares a slot's
   type (contract-file spec §5.5). A new legacy action, `legacy:body-type`, fails a reply labelled
   with a type that does not agree, with today's message (`Expected a body of 'text/plain' but the
   actual content type was 'text/html'`). Types agree when the reply has none, when either is
   unknown, when type and subtype are the same, or when the reply's suffix is the pact's subtype
   (`application/hal+json` agrees with `application/json`). The compiler emits the action only when
   the pact names no `Content-Type` header, since `legacy:header-value` already checks a header it
   names, and a matching rule on that header can widen what it accepts, as today.
5. **A corpus case may carry bytes.** `values` is JSON, so a case gains an optional `encoded` member:
   path to representation tag, with `base64` meaning the value at that path is the base64 of an
   octet sequence (engine-protocol spec §2.5). It is additive in plan-grammar v1.

## Alternatives considered

- **A catch-all `*/*` component that decodes anything as bytes.** It would keep decision 2 out of
  the kernel. But a requirement for a content type is satisfied by whatever claims it, so a contract
  that needs a CSV component nobody declared would stop failing as `component-unavailable`. That
  error is the point of declaring requirements.
- **Text and bytes handled in the legacy layer alone.** The v1–v4 path would work without a
  component. But a Janus contract with a `text/plain` body would still have nothing to decode or
  encode it, and S1.21 to S1.23 would each add a second, legacy-only reader beside the component
  that a contract needs anyway.
- **Decode the reply by its own label when it disagrees with the pact's type, and no type check.**
  The body then fails to match because it decodes as something else. But the same text under
  another text type, or JSON under a `+json` type the pact did not name, decodes the same and
  passes. S1.1's mutations relabel every reply's content type, and today's verifier fails each one.
- **Decode `charset`s other than UTF-8.** Today's verifier does not: it reads text as UTF-8 and
  compares bytes when there are no rules. Adding an encoding library for a case no fixture has would
  make Janus differ in the other direction. Staged plan S6.5 adds them later, once matching
  today's verifier no longer constrains the choice.

## Consequences

- A v1–v4 pact with a text or binary body verifies as it does today. The spec cases for plain text,
  including the two regex cases S1.18 anchored, run with no `janus:skip`.
- A Janus contract can declare a `text/` body that is a string and have it decoded and served, with
  no component declared. A structured shape under a `text/` type still needs a component, as it did
  before; S1.6 first shipped without that check, and the TypeScript SDK's CSV test caught it. `engine/hello` lists
  `content/text` among the in-tree components.
- An unclaimed body is now present, as bytes, wherever the engine decodes one, not only on the v1–v4
  path. A shape that asks for such a body gets bytes rather than nothing. That is truer, and a shape
  that needs the body read still needs a component, which a requirement still demands.
- A bytes mismatch names the first byte that differs, since the text forms show a bytes value as its
  length and at most 48 characters of its base64 (plan-grammar spec §3.1), and two large bodies that
  differ past that show the same prefix.
- A JSON pact with no `Content-Type` header now shows the type check in `explain`. It was always
  today's behaviour; the plan only says so now.
- Plan grammar v1 gains `legacy:body-type`, the first action v0 never had, so it has no v0 name.
- `janus upgrade` still carries a non-JSON body across with a `body-not-parsed` finding and asserts
  nothing about it. Converting text and bytes bodies into a contract's `content-types` and shapes is
  S1.25.
- The plan-grammar spec gains `legacy:body-type` (§4.4) and the corpus format's `encoded` (§6.1),
  and `corpus-case.schema.json` with it.
- **Tripwire**: if S1.21 to S1.23 cannot route their body kinds through decision 3's classification
  without a special case per kind in the legacy compiler, the classification belongs in the content
  components (a `legacy` view of their document), not in the legacy layer.
