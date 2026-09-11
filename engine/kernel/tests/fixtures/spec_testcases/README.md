# v1–v4 matching-rule spec test cases

Copied from [`pact-reference`](https://github.com/pact-foundation/pact-reference)
`rust/pact_matching/tests/spec_testcases/{v1,v1_1,v2,v3,v4}/{request,response}` at commit
`71841f3c277f7dcb1bf769c43c8a1494cf383476` (MIT licensed) — these mirror the
[pact-specification](https://github.com/pact-foundation/pact-specification) test cases and are the
behavioural oracle for plan task 3.5 (see `Documentation/reuse-inventory.md`). Message test cases
(`v3/message`, `v4/message`) are not copied: Janus does not compile message interactions yet.

Do not hand-edit these; if pact-specification adds/changes cases, re-copy from a recorded SHA rather
than diverging. The one sanctioned exception is a `"janus:skip": "<reason>"` member added to a
specific case's top level — not part of pact-specification's own schema, so it can't be mistaken for
upstream data — when this compiler is known to disagree with that one case's recorded verdict for a
reason specific to it (not a whole category of cases; those are excluded by a rule in
`legacy_matching.rs`'s `skip_reason` instead). Keep the reason concrete enough that someone re-reading
it later, with no memory of this session, can tell whether it's still true.

Each file is `{"match": bool, "comment": string, "expected": {...}, "actual": {...}}` — a request or
response fragment (method/path/query/headers/body/matchingRules, or status/headers/body/matchingRules)
in the v2 flat or v3/v4 categorized matching-rule JSON form. `engine/kernel/tests/legacy_matching.rs`
compiles `expected` (design 3.5) and executes the plan against `actual`, diffing the verdict against
`match` — unless `janus:skip` is present, in which case the case is excluded and reported separately.
