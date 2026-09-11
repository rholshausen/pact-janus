# v1–v4 matching-rule spec test cases

Copied from [`pact-reference`](https://github.com/pact-foundation/pact-reference)
`rust/pact_matching/tests/spec_testcases/{v1,v1_1,v2,v3,v4}/{request,response}` at commit
`71841f3c277f7dcb1bf769c43c8a1494cf383476` (MIT licensed) — these mirror the
[pact-specification](https://github.com/pact-foundation/pact-specification) test cases and are the
behavioural oracle for plan task 3.5 (see `Documentation/reuse-inventory.md`). Message test cases
(`v3/message`, `v4/message`) are not copied: Janus does not compile message interactions yet.

Do not hand-edit these; if pact-specification adds/changes cases, re-copy from a recorded SHA rather
than diverging.

Each file is `{"match": bool, "comment": string, "expected": {...}, "actual": {...}}` — a request or
response fragment (method/path/query/headers/body/matchingRules, or status/headers/body/matchingRules)
in the v2 flat or v3/v4 categorized matching-rule JSON form. `engine/kernel/tests/legacy_matching.rs`
compiles `expected` (design 3.5) and executes the plan against `actual`, diffing the verdict against
`match`.
