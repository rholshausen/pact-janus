---
default: minor
---

# `upgrade/pact` converts text, binary, form, XML and multipart bodies

Until now, `upgrade/pact` converted only JSON bodies. Any other body was carried across as the pact's octets
with a `body-not-parsed` finding, and nothing was asserted about it, so the contract verified less than its
pact (ADR 0047).

- **A body a built-in component reads declares its type.** A text body that is UTF-8, a form, XML and
  multipart get a `content-types` entry for their slot and a `requires` entry for `content/text`,
  `content/form`, `content/xml` or `content/multipart`. A multipart type is declared without its
  `boundary`. Each body's shape is over that component's document, built as the legacy compiler compares
  the body.
- **Octets stay undeclared**: a binary body, or text that is not UTF-8, is shaped as bytes with a base64
  `example` and `encoded: base64`.
- **A converted body's evidence is the pact's octets**: `encoded: text` when they are UTF-8, `base64`
  otherwise, with the pact's type in `content-type`. A replay sends them as they stand.
- **A pact that names no `Content-Type` header** gets a `content-type` member in that part's `headers`
  shape: `optional`, of a `regex` that admits what the pact's check of the reply's type admits. This
  applies to JSON bodies too, so a converted JSON interaction with no `Content-Type` header now has a
  `headers` shape, and a variant space of two.
- **A number or boolean rule on text** (a path, header, query or form value, an XML attribute or text)
  converts to the `regex` v1–v4 read the text by, where it converted to `integer`, `number` or
  `boolean`, which no string satisfies.
- **`eachValue` and `arrayContains`** convert with their own rules.
- **New finding codes**: `response-body-opened` (lossy), `body-type-on-header` (judgement) and
  `rule-as-pattern` (note). `body-not-parsed` is now raised only for a body that is not what its type says.
  `request-body-opened` and `rule-narrowed` are raised for the new body kinds too.

Two behaviours change outside `upgrade/pact`:

- **The built-in multipart component** decodes a slot whose declared type names no `boundary` at the
  boundary the slot's own label names.
- **The shape compiler quotes a member name a path cannot hold**, such as `a.b`, `''` or
  `@{urn:x}id`. Such a member used to be resolved as a different member, nested or missing, so a shape
  naming one matched the wrong value.
