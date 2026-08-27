# Worked example — the RFC's order payload, compiled and executed

[Design 2.2's worked example](../../shape-language/examples/order-payload.md) gives the canonical
shape for the RFC's order response and [design 2.3's](../../variant-semantics/examples/order-payload-sampling.md)
selects eight variants for it. This file is the third step: what the engine actually runs, and what a
user sees when it fails.

Every operator in §5.2 of the spec appears here at least once, which is the point — the compilation
table is a claim, and this is where the claim is legible.

## 1. The compiled plan

The response body's shape, compiled without a variant (spec §5.1), rendered in the pretty form
(spec §3.1):

```text
(
  :"get an order" (
    :response (
      :body (
        #{'decoded by the content component for application/json'},
        :"$.id" (
          %match:integer (
            $.response.body.id
          )
        ),
        :"$.status" (
          %match:any-of (
            $.response.body.status,
            'PENDING',
            'SHIPPED',
            'DELIVERED'
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
        ),
        :"$.payment" (
          %if (
            %check:equals (
              $.response.body.payment.type,
              'card'
            ),
            :card (
              :"$.payment.last4" (
                %match:regex (
                  $.response.body.payment.last4,
                  '\d{4}'
                )
              )
            ),
            %if (
              %check:equals (
                $.response.body.payment.type,
                'invoice'
              ),
              :invoice (
                :"$.payment.dueDate" (
                  %match:date (
                    $.response.body.payment.dueDate,
                    'yyyy-MM-dd'
                  )
                )
              ),
              %error (
                %join (
                  'Expected payment.type to be one of card, invoice but got ',
                  $.response.body.payment.type
                )
              )
            )
          )
        ),
        :"$.items" (
          %expect:size (
            $.response.body.items,
            1,
            NULL
          ),
          %for-each (
            ** (
              $.response.body.items
            ),
            :"$.items[*]" (
              :"$.items[*].sku" (
                %match:string (
                  ~>.sku
                )
              ),
              :"$.items[*].qty" (
                %match:integer (
                  ~>.qty
                )
              )
            )
          )
        )
      )
    )
  )
)
```

Five things in that tree are the design decisions of §4 and §5 made visible:

- **Value operators are one action; structural operators are structure.** `integer`, `any-of`,
  `datetime`, `regex`, `date` and `string` each became a single `%match:` node. `optional`, `one-of`
  and `each-like` became `%if`, nested `%if`, and `%for-each` — the presence decision, the
  discriminator decision and the iteration are nodes a reader can see and a result can be attached to
  (spec §4.3).
- **Every condition is a `check:`, never a `match:`.** `%check:exists` and `%check:equals` yield
  booleans. Putting `%match:equality` in the condition of the `payment` branch would fail the whole
  interaction while merely deciding which alternative to take (spec §4.2).
- **`one-of`'s failure names the value it read.** The innermost `%error` is what a user gets for
  `"type": "cheque"`, and it is a node in the plan rather than a message the interpreter invents.
- **`must-ignore` is an absence.** Nothing in `:body` mentions members the shape did not name, so
  `warehouse` and `network` are admitted because no node looks at them. That is the most inspectable
  form ADR 0007's commitment 4 could take (spec §5.2).
- **The one namespaced thing is annotated, not executed.** JSON decoding happens in the content
  component; the plan carries an `#{...}` annotation saying so. The kernel's plan never learned what
  JSON is (spec §4.6).

## 2. Compiled under a variant

Under the variant `response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED`
(design 2.3), the shape is narrower and so is the plan (spec §5.1). Two subtrees change:

```text
        :"$.status" (
          %match:equality (
            $.response.body.status,
            'SHIPPED'
          )
        ),
        :"$.shippedAt" (
          %expect:absent (
            $.response.body.shippedAt
          )
        ),
```

The `any-of` pinned to one point is an equality; the `optional` pinned to `absent` is an assertion,
not a branch. This is what "matching against a variant is narrower" (shape spec §7.1) looks like once
compiled — the variant did not filter results after the fact, it changed what the engine runs.

## 3. Executed, and failing

The same plan (§1) against a provider response with `"status": "REFUNDED"`, an invoice payment whose
`dueDate` is a timestamp, and an empty `items` array. Executed form (spec §3.2), trimmed to the
subtrees that matter:

```text
        :"$.status" (
          %match:any-of (
            $.response.body.status => 'REFUNDED',
            'PENDING',
            'SHIPPED',
            'DELIVERED'
          ) => ERROR(Expected 'REFUNDED' to be one of 'PENDING', 'SHIPPED', 'DELIVERED')
        ) => BOOL(false),
        :"$.payment" (
          %if (
            %check:equals (
              $.response.body.payment.type => 'invoice',
              'card'
            ) => BOOL(false),
            :card (
              :"$.payment.last4" (
                %match:regex (
                  $.response.body.payment.last4,
                  '\d{4}'
                )
              )
            ),
            %if (
              %check:equals (
                $.response.body.payment.type => 'invoice',
                'invoice'
              ) => BOOL(true),
              :invoice (
                :"$.payment.dueDate" (
                  %match:date (
                    $.response.body.payment.dueDate => '2026-08-30T00:00:00Z',
                    'yyyy-MM-dd'
                  ) => ERROR('2026-08-30T00:00:00Z' is not a date in format 'yyyy-MM-dd')
                ) => BOOL(false)
              ) => BOOL(false)
            ) => BOOL(false)
          ) => BOOL(false)
        ) => BOOL(false),
        :"$.items" (
          %expect:size (
            $.response.body.items => [],
            1,
            NULL
          ) => ERROR(Expected at least 1 item but got 0),
          %for-each (
            ** (
              $.response.body.items => []
            ) => []
          ) => BOOL(true)
        ) => BOOL(false)
```

Three things this shows that a list of mismatches would not:

- **The `:card` subtree has no `=> ` anywhere.** It was not executed, because `%if` is lazy (spec
  §2.4) and its condition was false. A reader can tell "skipped" from "passed" at a glance, which is
  exactly why laziness has to be declared rather than incidental.
- **Every failure was found in one run.** The status, the due date and the item count all report;
  execution did not stop at the first error, because an error is a value (spec §2.3).
- **The `%for-each` succeeded.** Zero elements means the body ran zero times, which is true and not
  useful — the cardinality assertion beside it is what carries the real verdict. Splitting them is
  what lets the report say "0 items" rather than "no failures in the loop".
