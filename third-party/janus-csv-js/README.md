# janus-csv-js

The `csv` content component for `text/csv`, written in JavaScript with no dependencies, and reached
through the subprocess binding (component-interfaces spec §9.3, staged plan S5.2b,
[ADR 0108](../../Documentation/decisions/0108-a-subprocess-component-is-a-program-and-its-arguments-and-its-process-is-the-unit-of-recreation.md)).

It is [`janus-csv`](../janus-csv) written again in another language, from the published component
interfaces. It answers the same handshake, and it is held to the Rust component's own conformance
cases, [`../janus-csv/conformance/`](../janus-csv/conformance), so those cases say what a CSV component
does rather than what one implementation of it happens to do.

- [`pipe.mjs`](pipe.mjs) is the component's end of the pipe: `Content-Length` frames on stdin and
  stdout, logs on stderr, exit when stdin ends. It is the whole of what an author in another language
  writes before their first operation.
- [`csv.mjs`](csv.mjs) is the component: `content/decode`, `content/encode`, `content/compile`, and
  `matcher/apply` and `matcher/compare` for the actions its handshake substitutes and its `csv:column`
  operator.

Declare it in a verifier configuration, beside which it starts:

```yaml
components:
  - name: csv
    source: { kind: subprocess, reference: node, args: [path/to/janus-csv-js/csv.mjs] }
```

and run the cases through the binding a run loads it with:

```bash
janus component check --config verifier.janus.yaml csv third-party/janus-csv/conformance
```

It needs Node 18 or later. The tests that load it are `engine/component-host/tests/subprocess.rs` and
`cli/tests/cli.rs`, which skip without Node unless `CI` is set.

One difference from the Rust component is JavaScript's, not CSV's: a JavaScript object puts a member
named like an array index (`"0"`, `"42"`) first, so `encode` writes such a column first in the header.
`decode` keeps the header's order, whatever the names.
