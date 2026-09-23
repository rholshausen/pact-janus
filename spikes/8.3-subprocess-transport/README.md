# Spike 8.3 — The out-of-process transport escape hatch

Plan task 8.3. **The deliverable is [FINDINGS.md](FINDINGS.md)**; the code here is disposable.

## The question

Component-interfaces spec §9.3 specifies a third binding for the cases a WASM sandbox cannot serve:
a component the engine spawns, speaking the same frames over the Engine Protocol's own
`Content-Length` stdio framing — not gRPC ([ADR 0013](../../Documentation/decisions/0013-component-hosting-is-an-embedding-capability.md)).
Nothing had ever used it. The plan asks for three things of it:

1. **Does the boundary exist?** A transport running out of process, driven by the real engine, as a
   consumer mock and as a verifier — today's pact-plugins architecture on this project's wire.
2. **What do grants cost when they cannot be enforced?** Spec §9.3 says they cannot.
3. **Does spike 1.3's Windows risk apply to component processes?**

The stretch — a gRPC *protocol* transport over the same pipe — was not attempted (FINDINGS §5).

## What is here

| Path | What |
|---|---|
| `src/lib.rs` | `SubprocessLoader`: the engine side of the binding — spawn, framing, correlation by id, deadlines with kill escalation, respawn, `env` enforcement |
| `src/bin/host.rs` | a host process for the orphan test |
| `component/pipe.mjs` | the component side of the framing, in Node, dependency-free: the "30–60 lines" claim |
| `component/tcp-transport.mjs` | a `tcp` transport — JSON lines over a raw socket, both roles |
| `component/misbehaving.mjs` | a component that hangs, spins, exits, emits garbage, prints to stdout, reports its environment |
| `tests/transport.rs` | the engine end to end: consumer mock, mismatch, verification pass and fail |
| `tests/containment.rs` | the binding's obligations, against `misbehaving.mjs`, and what grants enforce |
| `examples/call_cost.rs` | spawn, pipe and exchange costs |

The kernel half — a declared component contributing a transport, and passive exchanges armed by the
interaction's transport kind instead of `"http"` — is not spike code: it is in `engine/kernel`, tested
by `engine/kernel/tests/declared_transport.rs`.

## Running it

Needs Node on `PATH` (22 was used). From this directory:

```bash
cargo test                                   # 14 tests
cargo run --release --example call_cost      # the numbers in FINDINGS §3
```

CI runs both test files on Linux and Windows (`spike-8-3` in `.github/workflows/ci.yml`).
