---
engine: minor
---

# A slot that cannot be decoded fails at the slot, a decode's losses are warnings, and encode is given the shape

Staged plan S5.3, ADR 0101. What a content component says about a body now reaches the user.

- **A body a component refuses fails once, at the body, in the component's words**: `$.response.body:
  could not be decoded as 'application/json': expected value at line 1 column 1`, with no `action`.
  No check that reads the body runs against a guess, so the mismatches that used to follow it
  (`Expected a value`, `Expected at least 1 item(s)`) are gone. Checks that read anything else, the
  status or a header, still run. An empty body is still absent, not undecodable. The verdict of every
  run is the one it was; the messages are new.
- **A content component that died decoding a reply** (trapped, timed out, exited) fails the variant
  with `component-failed`, naming `content/<type>` and the slot's `path`, as a transport's failure does.
- **New `verification/warning` codes**, before the result they qualify: `content-degraded`, a loss a
  decode reported, at its place in the reply (`target`), with the component's `degradation`; and
  `content-type-differs`, a slot a contract declares one type for that arrived labelled with another
  (`declared`, `labelled`) and was decoded as declared. Neither changes a verdict. Reports print them.
- **`content/encode` takes an optional `shape`**, the slot's shape as the interaction declares it,
  whenever the engine produced the document from one. A component writes from it only what the
  document cannot say, such as the header row of an empty CSV. Components that ignore it are unchanged.
- The golden corpus's `case.json` may name `undecodable` slots.
