# Golden corpora

Executable-specification artifacts: for each case, an input (interaction spec or v1–v4 pact), the
expected compiled plan, and the expected result of executing that plan against captured values. The
corpus format is defined by the plan-grammar design (plan task 2.4); cases land from Phase 3 (task 3.7).

**Corpora are load-bearing**: any change to matching behaviour must change this directory in the same
commit, and CI executes plans against these cases on every change.
