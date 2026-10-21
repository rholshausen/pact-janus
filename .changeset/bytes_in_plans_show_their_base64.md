---
default: patch
---

# A bytes value in a plan shows its base64

`explain`, the executed form and mismatch messages printed a bytes value as `<8 byte(s)>`, which said
nothing about what the bytes were. They now print `BYTES(8, iVBORw0KGgo=)`: the length, then the
base64, as the forked engine did. The base64 is cut after 48 characters (the first 36 bytes) and ends
in `...` when it is longer (plan-grammar spec §3.1). A corpus that pinned the old form must accept the
new one.
