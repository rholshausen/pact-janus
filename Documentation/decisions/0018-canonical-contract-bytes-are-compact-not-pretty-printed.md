# 0018 — Write Janus contracts as compact JSON; canonical form carries no formatting promise

- **Status**: accepted
- **Date**: 2026-09-06
- **Plan tasks**: 3.1 (revises [ADR 0011](0011-contracts-as-self-identifying-json-documents.md) decision 4)
- **Evidence**: task 3.1's first working writer (`engine/kernel/src/contract/write.rs`), built exactly
  as ADR 0011 decision 4 specified

## Context

ADR 0011 decision 4 said writing is canonical: "UTF-8 without BOM, LF, two-space indent, one specified
member emission order, ... trailing newline, no trailing whitespace." Decision 3, in the same ADR,
requires a conformant file's literal first eleven bytes to be `{"$format":`.

Building both literally, in task 3.1, showed they conflict. Every JSON pretty-printer — `serde_json`'s
included — writes a newline and the first level of indent immediately after the opening `{`, so a
pretty-printed document's first bytes are `{\n  "$format": ...`, not `{"$format":...`. The writer worked
around this by serialising pretty, then splicing the first two lines into a hand-built compact opening
for `$format` alone, and resuming normal indentation for everything else.

That splice is a cost with no reader on the other end of it. No parser cares about whitespace — `read()`
already parses arbitrary formatting, because that is what a JSON deserializer does — so the pretty form
buys nothing machine-facing. What it buys a human is a file they can read without a formatting step, and
that is a real want, but baking it into the writer means every future implementation of a Janus writer
(the JVM and TypeScript SDKs, not just this prototype's Rust one) has to reproduce the exact same
one-member special case to stay byte-compatible, forever, for a convenience `jq -S` (or any editor's
format-on-save) gives for free and losslessly, on demand, at read time instead of write time.

## Decision

**A canonical Janus contract is compact JSON: no insignificant whitespace at all.** No space after `:`
or `,`, no newlines except the one trailing the document. Everything else ADR 0011 decision 4 already
said stands unchanged: UTF-8, no BOM, LF-only, member order as specified (`$format` first), arrays in
their specified order, a single trailing newline, no trailing whitespace.

`$format` first plus compact output makes the eleven-byte prefix `{"$format":` fall out with no special
case: there is nothing between `{` and the first key for a compact writer to insert. Decision 3's
identification rule needs no change and no splice to satisfy.

**A reader MUST NOT assume its input is canonically formatted.** This was already true of the JSON parse
itself — whitespace, indentation and (beyond `$format` being first) member order have never affected
what a standard JSON deserializer produces — but it is worth stating as its own rule now that the writer
has an opinion about its own output: compactness is a promise about what *this* writer emits, never an
assumption a reader may make about *any* input. A contract that has round-tripped through a broker, git,
a formatter, or a hand edit is exactly as readable as one straight from this writer; `IdentifyMode::Tolerant`
(contract-file spec §2.3) exists precisely for the case where formatting is no longer this writer's own.

## Alternatives considered

- **Keep the splice** (status quo). Rejected: it is bespoke logic for one member's formatting that buys
  nothing for a machine reader, and every SDK's own writer has to re-derive it independently to stay
  byte-compatible — a much larger footprint than one ADR revision.
- **Pretty-print everything and drop the byte-prefix promise**, falling back to tolerant identification
  always. Rejected: strict identification is explicitly valuable for "pipelines that control their
  writers" (contract-file spec §2.3); a compact writer keeps both modes cheap without a special case.
- **Two canonical forms** (compact for machines, pretty for a `--pretty` flag or a review artifact).
  Rejected here, not forever: nothing stops a CLI command from pretty-printing a contract for display —
  that is a presentation concern, not a second canonical byte sequence, and determinism (ADR 0011
  decision 4's whole point) needs exactly one canonical form to dedupe and diff against.

## Consequences

Contract files on disk are not directly readable without a formatting step; anywhere this repo's own
tooling shows a contract's bytes to a human (a future `explain`/`upgrade` rendering, docs) formats at
display time and never assumes the file already looks that way. The worked examples under
`Documentation/specs/contract-file/examples/*.md` stay pretty-printed in the docs — that is what
"reformatted for review" means — and were never a claim about literal file bytes.

Committed to: exactly one canonical byte sequence per contract, so content-addressed dedup and diffs
stay meaningful; a writer that ever reads back its own output and gets a different `Contract` value, or
writes the same `Contract` twice and gets different bytes, has broken this ADR, not chosen a stylistic
variant of it.
