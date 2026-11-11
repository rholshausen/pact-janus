---
engine: major
---

# v1–v4 read parameters in `Accept` and `Content-Type` only, and `janus upgrade` writes an `Accept` as a media type

Staged plan S3.4, ADR 0083.

- **A v1–v4 header with no rule is compared as today's verifier compares it.** `Accept` and
  `Content-Type` compile to `legacy:header-value`, which compares each comma-separated element with the
  one at its index, as media types where either has parameters. So `Accept: application/json,
  text/plain` now passes a reply's `application/json;q=1, text/plain`, as it does today. Every other
  header compiles to a new legacy action, `legacy:header-text`: its text, ignoring whitespace around a
  comma, a parameter compared as any other text. So `X-Params: a;b=1` now fails a reply's
  `a;b=1;c=2`, as it does today, where `janus verify` passed it.
- **`janus upgrade`** writes an `Accept` of one media type as the HTTP component's `http:media-type`,
  as it writes a `Content-Type`, so the contract passes a quality or a charset in another case that its
  pact passes. The interaction requires `http`, with a `rule-narrowed` note. The `rule-narrowed`
  judgement on headers is left only on an `Accept` of several elements, which is `http:list`; another
  header with parameters is converted with no judgement, since nothing is narrowed.
- **Plan grammar.** `legacy:header-text` is a legacy-only action taking two children, the value and the
  expected text. A plan compiled from a pact with a header other than `Accept` or `Content-Type` names
  it where it named `legacy:header-value`.
