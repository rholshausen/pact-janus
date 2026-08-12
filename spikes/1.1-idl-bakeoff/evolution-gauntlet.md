# The evolution gauntlet

Five representative interface changes, applied to the protocol slice. Each is tested in two
directions. "Artifact" means a compiled thing that cannot be rebuilt — an old plugin binary, an SDK
release already on someone's machine.

## Changes

| # | Change | Slice instance |
|---|--------|----------------|
| E1 | Add an enum case | `error-code` gains `component-unavailable` |
| E2 | Add an optional field to a record/message | `engine-error` gains `retryable: option<bool>` |
| E3 | Add a new operation | interface gains `explain(spec) -> result<plan-text, engine-error>` |
| E4 | Add a variant case to an event stream | `verify-event` gains `hook-invoked(...)` |
| E5 | Widen a payload (new alternative in a union/oneof) | `interaction-part` gains a `message` alternative alongside `request`/`response` |

## Directions

- **D-a (critical, plugin surface)**: artifact built against interface *v N*, counterpart speaks
  *v N+1*. The old plugin must load and work; new cases it never sees must simply not reach it, or
  reach it in a form it can safely ignore.
- **D-b (important, SDK surface)**: artifact built against *v N+1*, counterpart speaks *v N*. Graceful,
  detectable degradation is required — a clean "engine too old for feature X" beats a link error, which
  beats silent misbehaviour.

## Scoring, per (candidate × change × direction)

- **PASS** — works without recompilation; unknown data is ignorable or detectable by design.
- **NEGOTIATED** — works, but only via an explicit mechanism the design must adopt (feature gates,
  capability flags, version negotiation). Record the mechanism's cost.
- **RECOMPILE** — old source rebuilds fine but old artifacts break. Fails the plugin surface.
- **BREAK** — type/link/runtime failure of the old artifact. Disqualifying for the plugin surface;
  heavy penalty for the SDK surface.

Also recorded per candidate: what the *failure looks like* (compile error, instantiation error,
runtime panic, silent skip) — a loud early failure scores above a silent one within the same grade.
