---
default: patch
---

# A request the provider could not answer is reported once, on one line

`janus verify` printed a variant's `component-failed` error with its details as a JSON line under
it, and the engine logged the same failure again at `warn`, in Rust's debug format. Now the line
ends with the component's own code in brackets (`… frame with invalid size [transport-failed]`),
which is what the JSON added, and `--output json` still carries the whole error. The engine logs a
variant's failure, a transport failure or mismatches, at `debug`, since the run's result event
already reports it.
