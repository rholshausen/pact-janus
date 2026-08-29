# Worked example — a v3 pact converted, and what it costs

The claim this conversion has to support is the one in the project plan: *providers upgrade first, at
no cost to their consumers*. A provider adopts Janus, points it at the v3 pacts its consumers already
publish, and verifies. Nothing on the consumer side changes.

The claim it must **not** make is that conversion is lossless. It is not, and the point of
[`upgrade-findings.schema.json`](../schemas/v1/upgrade-findings.schema.json) is that every place it
loses something says so out loud. This file walks one pact through both halves.

Blocks marked `sketch` are the source pact — a v3 document, owned by the Pact specification and not by
anything here. The ```` ```json contract ```` and ```` ```json findings ```` blocks are validated
against this design's schemas by `cargo test -p pact_janus_schema_compat`.

## 1. The source pact

```json sketch
{ "consumer": { "name": "orders-ui" },
  "provider": { "name": "orders-api" },
  "interactions": [
    { "description": "get an order",
      "providerStates": [ { "name": "an order exists", "params": { "id": "42" } } ],
      "request": { "method": "GET", "path": "/orders/42" },
      "response": {
        "status": 200,
        "headers": { "Content-Type": "application/json" },
        "body": { "id": 42, "status": "PENDING", "total": 10.0,
                  "callbackUrl": "http://localhost:1234/orders/42",
                  "customer": { "id": "C-7", "name": "Ada" } },
        "matchingRules": {
          "header": { "Content-Type": { "matchers": [ { "match": "regex", "regex": "application/json.*" } ] } },
          "body": {
            "$.id": { "matchers": [ { "match": "integer" } ] },
            "$.status": { "matchers": [ { "match": "regex", "regex": "PENDING|SHIPPED" } ] },
            "$.total": { "matchers": [ { "match": "number" },
                                       { "match": "regex", "regex": "^\\d+\\.\\d+$" } ],
                         "combine": "AND" },
            "$.customer.id": { "matchers": [ { "match": "type" } ] } } },
        "generators": {
          "body": { "$.id": { "type": "RandomInt", "min": 1, "max": 100 },
                    "$.callbackUrl": { "type": "MockServerURL",
                                       "example": "http://localhost:1234/orders/42",
                                       "regex": ".*(\\/orders\\/42)$" } } } } } ],
  "metadata": { "pactSpecification": { "version": "3.0.0" } } }
```

Five constructs, four outcomes. `$.id`, `$.status` and `$.customer.id` map straight across.
`$.customer.name` has no rule at all. `$.total` carries two matchers combined with `AND`.
`$.callbackUrl` carries a generator that means something to a mock server and nothing to a shape.

## 2. The contract it becomes

```json contract
{ "$format": "janus-contract/1",
  "consumer": { "name": "orders-ui" },
  "provider": { "name": "orders-api" },
  "interactions": [
    { "description": "get an order",
      "transport": { "kind": "http", "mode": "passive" },
      "states": [ { "name": "an order exists", "params": { "id": "42" } } ],
      "parts": {
        "request": {
          "method": { "shape": "equality", "example": "GET" },
          "path": { "shape": "equality", "example": "/orders/42" } },
        "response": {
          "status": { "shape": "equality", "example": 200 },
          "headers": { "shape": "object",
                       "members": { "Content-Type": { "shape": "each-like",
                                                      "items": { "shape": "regex",
                                                                 "pattern": "application/json.*",
                                                                 "example": "application/json" } } } },
          "body": { "shape": "object",
                    "members": {
                      "id": { "shape": "integer", "example": 42,
                              "generator": { "type": "RandomInt", "min": 1, "max": 100 } },
                      "status": { "shape": "regex", "pattern": "PENDING|SHIPPED", "example": "PENDING" },
                      "total": { "shape": "number", "example": 10.0 },
                      "callbackUrl": { "shape": "equality", "example": "http://localhost:1234/orders/42" },
                      "customer": { "shape": "object",
                                    "members": { "id": { "shape": "type", "example": "C-7" },
                                                 "name": { "shape": "equality", "example": "Ada" } } } } } } },
      "selection": {
        "variants": [
          { "id": "base", "origin": "base", "assignment": [],
            "states": [ { "name": "an order exists", "params": { "id": "42" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "headers": { "content": { "Content-Type": ["application/json"] } },
                            "body": { "content": { "id": 42, "status": "PENDING", "total": 10.0,
                                                   "callbackUrl": "http://localhost:1234/orders/42",
                                                   "customer": { "id": "C-7", "name": "Ada" } },
                                      "encoded": "json", "content-type": "application/json" } } } } ],
        "report": {
          "space": { "size": 1, "exact": true, "dimensions": 0 },
          "strategy": "exhaustive", "algorithm": "janus-ipog-v1", "selected": 1,
          "coverage": { "targets": 0, "covered": 0, "removed": 0, "dropped": 0 },
          "boundaries": false } } } ],
  "metadata": { "writer": { "pact-janus": "0.1.0" } } }
```

No branch anywhere produced that selection. The shape contributes no variant dimensions, so the space
has one member, every strategy agrees on it, and the base variant is the whole selection — the
degenerate case falling out of the arithmetic rather than being special-cased around (variant semantics
§9, spec §8.3).

## 3. The findings

```json findings
{ "findings": [
  { "code": "rule-unmapped", "kind": "lossy",
    "path": "/interactions/0/response/matchingRules/body/$.total",
    "target": "/interactions/0/parts/response/body/members/total",
    "message": "Two matchers ('number' and a regex) are combined with AND and neither is narrower than the other. The shape language has no intersection operator, so the first is kept and the regex is dropped: a provider returning 10 rather than 10.0 will now pass. If the decimal places matter, replace the pair with a single regex." },
  { "code": "generator-dropped", "kind": "lossy",
    "path": "/interactions/0/response/generators/body/$.callbackUrl",
    "target": "/interactions/0/parts/response/body/members/callbackUrl",
    "message": "A MockServerURL generator describes the mock server, which a shape has no notion of. The recorded example is kept as an equality constraint; if the provider returns its own base URL there, this interaction will now fail on it." },
  { "code": "example-frozen-as-equality", "kind": "note",
    "path": "/interactions/0/response/body/customer/name",
    "target": "/interactions/0/parts/response/body/members/customer/members/name",
    "message": "No matching rule applied here, so the example became an equality constraint. That is what the pact already meant; it is only more visible now." },
  { "code": "state-params-untyped", "kind": "note",
    "path": "/interactions/0/providerStates/0/params",
    "target": "/interactions/0/states/0/params",
    "message": "State parameters carried across as written. The pact recorded id as the string \"42\"; nothing here decides whether the provider wants a string or a number, and nothing guesses." } ] }
```

## 4. What to notice

**The two `lossy` findings are the whole value of the mechanism.** Both make the contract *weaker* than
the pact, and both are the kind of change that would otherwise be discovered months later as a
verification that passes when it should not. `rule-unmapped` says so and names the fix; `generator-dropped`
says so and names the failure it will cause instead. Neither is silent, and neither blocks the upgrade.

**`example-frozen-as-equality` is a `note`, not a finding to act on.** A position with no matching rule
already meant equality in v1–v4; the conversion changed nothing, it only wrote down what was already
true. Reporting it as lossy would train people to ignore the list, which is the failure mode a findings
mechanism has.

**The `kind` split is doing real work here.** A single severity would have to rank "the contract now
accepts more than it did" against "you should know the converter made a choice", and those are not
points on one scale.

**Nothing here needed a v5 of anything.** The source stays a valid v3 pact, owned by the Pact
specification; the output is a Janus contract, owned by this project. That separation is
[ADR 0011](../../../decisions/0011-contracts-as-self-identifying-json-documents.md), and this file is
the case it was written for.
