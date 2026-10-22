---
default: patch
---

# Generated bindings list members in the order the schema writes them

`janus-sdk-gates bindings generate` now emits each type's members in the order its schema declares them,
however it was built. Before, the order depended on what was compiled alongside the generator. Built on its
own it sorted them alphabetically. Built in the release, next to the CLI, it inherited the kernel's
member-order setting (ADR 0037) and kept the schema's order. The bindings a release's gates generated
therefore differed from the ones checked in, so the release's bindings gate failed.

An SDK that checks its bindings against a release's gates regenerates them once. The change is member
order only: the JVM classes' `@JsonPropertyOrder` follows the schema, so they serialise members in that
order. No type, member or constant changes.
