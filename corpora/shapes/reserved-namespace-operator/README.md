# An operator in a reserved namespace is refused

Staged plan item S0.13. The corpus's one case with no plan: `status: invalid`, so there is no
`plan.txt` or `executed.txt` beside it (plan-grammar spec §6.1).

A component operator compiles to the action of the same name: `csv:integer` in a shape becomes
`%csv:integer` in the plan, run by whichever component contributes it. That rule had a hole. An
operator spelled in a namespace the plan grammar reserves for its core actions (`flow`, `value`,
`expect`, `check`, `match`, `legacy`) became that core action, handed the shape's example and members
as arguments it was never meant to take. Before this case existed:

- `flow:if` and `value:length` checked nothing, and reported a match whatever the provider sent;
- `legacy:only-entries` put a closed-object check in a shape, which ADR 0007 commitment 4 refuses;
- `expect:size` and `legacy:min-max-type` read a child that was not there, and the interpreter
  panicked.

No component can be named for a reserved namespace, so no such operator can ever be resolved. The
engine says so before it compiles anything: `interaction-invalid`, one problem per operator, each at the
node and naming the operator. The `id` member is an ordinary `integer` and is not among the problems —
the refusal is about the six operators, not about the document they are in.
