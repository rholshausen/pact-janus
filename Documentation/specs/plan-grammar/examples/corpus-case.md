# Worked example — a golden-corpus case

A corpus case is four files in a directory (spec §6.1). This is one of them end to end, and then the
two ways it can go red — which mean opposite things and must be told apart on sight.

The `case.json` below is validated against
[`corpus-case.schema.json`](../schemas/v0/corpus-case.schema.json) by
`cargo test -p pact_janus_schema_compat`.

## 1. `case.json`

```json corpus
{ "description": "optional member, absent: the presence branch is skipped and the interaction passes",
  "input": {
    "kind": "spec",
    "spec": {
      "description": "a request for an order",
      "response": {
        "body": {
          "shape": "object",
          "members": {
            "id": { "shape": "integer", "example": 42 },
            "shippedAt": { "shape": "optional",
                           "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                                   "example": "2026-07-30T10:00:00Z" } } } } } } },
  "values": {
    "$.response.body": { "id": 7, "warehouse": "AKL-1" } },
  "result": { "status": "matched" } }
```

`values` is the whole reason a corpus case runs offline: no mock server, no provider, no transport.
The captured value stands in for whatever a resolver would have supplied (spec §2.4), so the case
tests the compiler and the interpreter and nothing else.

Note `warehouse`. It is in the captured value and named nowhere in the shape, and the expected result
is `matched` — this case is *also* the regression test for must-ignore (shape spec §4.3), which is a
guarantee no assertion in the plan can state, only the absence of one.

## 2. `plan.txt`

```text
(
  :"a request for an order" (
    :response (
      :body (
        :"$.id" (
          %match:integer (
            $.response.body.id
          )
        ),
        :"$.shippedAt" (
          %if (
            %check:exists (
              $.response.body.shippedAt
            ),
            %match:datetime (
              $.response.body.shippedAt,
              'yyyy-MM-dd\'T\'HH:mm:ssX'
            )
          )
        )
      )
    )
  )
)
```

## 3. `executed.txt`

```text
(
  :"a request for an order" (
    :response (
      :body (
        :"$.id" (
          %match:integer (
            $.response.body.id => 7
          ) => BOOL(true)
        ) => BOOL(true),
        :"$.shippedAt" (
          %if (
            %check:exists (
              $.response.body.shippedAt => NULL
            ) => BOOL(false),
            %match:datetime (
              $.response.body.shippedAt,
              'yyyy-MM-dd\'T\'HH:mm:ssX'
            )
          ) => BOOL(true)
        ) => BOOL(true)
      ) => BOOL(true)
    ) => BOOL(true)
  ) => BOOL(true)
)
```

The `%match:datetime` node has no result. It was never executed, because the member is absent and
`%if` is lazy. A corpus that recorded only the verdict would record `matched` and lose the fact that
the interesting branch is the one that *didn't* run — which is precisely the evidence this case
exists to preserve.

## 4. Red of the first kind: a result diff

Suppose a change to the `optional` compilation makes an absent member fail. The run reports:

```text
✗ corpora/shapes/optional-absent
  result: expected 'matched', got 'mismatched'
    $.response.body.shippedAt: Expected a datetime but the member was absent

  This is a behaviour change. Either it is a bug, or matching behaviour changed
  deliberately — in which case case.json changes in this commit.
```

**A `result` diff is never an acceptable optimisation.** The engine now rejects something it used to
accept, and `CLAUDE.md`'s rule applies: a behaviour change without a corpus change is a bug, and a
corpus change is where the deliberate ones get argued for.

## 5. Red of the second kind: a snapshot diff

Now suppose a compiler improvement notices that `$.response.body.shippedAt` is resolved twice — once
to decide the branch and once to match — and hoists it into a pipeline so the resolver runs once:

```text
        :"$.shippedAt" (
          -> (
            $.response.body.shippedAt,
            %if (
              %check:exists (
                ~>
              ),
              %match:datetime (
                ~>,
                'yyyy-MM-dd\'T\'HH:mm:ssX'
              )
            )
          )
        )
```

The run reports:

```text
~ corpora/shapes/optional-absent
  plan.txt differs; result unchanged ('matched', 0 mismatches)
    + -> (
    +   $.response.body.shippedAt,
        %if (
          %check:exists (
    -       $.response.body.shippedAt
    +       ~>
    ...
  Regenerate with `cargo run -p pact_janus_corpus -- accept` and review the diff.
```

**This is permitted** (spec §7.1): a newer engine may compile a different plan for the same input. But
it is not silent, and that is the whole design. The plan text is what `explain` prints, so hoisting a
resolve changes what every user of this shape sees when they debug — a user-visible change that a
verdict-only corpus would have waved through.

What makes it *safe* rather than merely permitted is the half that did not change: the verdicts over
the captured values still hold. That is evidence, not proof — deciding whether two plans accept the
same set of values is the subsumption problem over a richer language than shapes, and out of reach
(spec §6.3). Which is why the case's value is only as good as its captured values, and why task 3.7's
job is choosing them rather than generating them.

## 6. The two-path case

A v1–v4 pact can reach a plan two ways: compiled directly by the legacy compiler (design 3.5), or
upgraded to shapes (design 2.5) and compiled as shapes. Spec §4.4 requires them to agree on verdicts,
and `also-compiled-from` is where a case checks it:

```json corpus
{ "description": "v3 type matcher on an integer agrees whether compiled directly or via upgrade",
  "input": {
    "kind": "pact-interaction",
    "pact": { "consumer": { "name": "c" }, "provider": { "name": "p" } },
    "index": 0 },
  "also-compiled-from": {
    "kind": "spec",
    "spec": { "response": { "body": { "shape": "integer", "example": 42 } } } },
  "values": { "$.response.body": 7 },
  "result": { "status": "matched" } }
```

Both inputs run; both must reach `matched`. Their **plans may differ** — the legacy compiler and the
shape compiler are allowed to build different trees — and only `result` is asserted across the pair.
Two compilers agreeing is the entire migration promise, and an unchecked promise about migration is
how migrations break.
