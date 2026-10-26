---
default: minor
---

# A mismatch is located at the element that failed, and an absent value prints as `ABSENT`

Staged plan S1.30: what the engine reports, from the review of the CLI's output.

- **Every mismatch path is absolute.**
  - One inside a `flow:for-each` template, a `match:contains` entry or a `legacy:array-contains`
    variant is located at the element or entry it was about: `$.response.body.prices.plum`, not
    `~>.value`, and `$.request.body.animals[1].phoneNumber`, not `~>.phoneNumber`.
  - An entry's `.key` and `.value` are located at the entry's member.
  - A mismatch from an action whose value under test is itself computed, such as `expect:size` over
    `legacy:xml-children(...)` or `match:equality` over `value:upper-case(...)`, is located where that
    value came from. Until now it had no path.
  - The plan-grammar spec's §2.3 states the rule. A host that showed relative paths needs no change.
- **The executed text form writes an absent value as `ABSENT`**, and a `null` as `NULL`; both used to
  be `NULL` (plan-grammar spec §3.2). The pretty form is unchanged: no value node can be absent.
- **A container for a member no plain path segment can hold is labelled as its resolve names it**:
  `:"$['']"` and `:"$['a.b']"`, not `:"$."` and `:"$.a.b"`. A label at a slot's root is shortened as
  every other is (`:"$[0]"`, not `:"$.response.body[0]"`).
- **Messages:** "Expected an integer" and "an array", not "a integer" and "a array". A count says
  "1 item" or "2 items", and `subsumption/decide`'s report text, `filter-matches-nothing` and the pact
  plugin refusal use plurals instead of `(s)`.
