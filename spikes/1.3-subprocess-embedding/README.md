# Spike 1.3 — Subprocess embedding

Plan task 1.3 · Feeds gate G1 · Findings in [FINDINGS.md](FINDINGS.md) (the durable artifact — code
here is disposable).

## Question

The subprocess embedding is the fallback that makes "WASM preferred" (B1) safe to assert — so it
must be *shown* to work early, not assumed. Concretely: can a test runner in each SDK language
spawn a `pact-engine` executable, negotiate a protocol version, drive it over stdio with LSP-style
framing, and shut it down cleanly — and does the engine reliably die when the test runner is
killed (no orphaned processes)? What does the pipe cost versus 1.2's in-process WASM numbers?

## Method

1. **Engine binary** (`engine-bin/`): the 1.2 toy engine logic (copied in — spikes stay
   standalone) behind an LSP-style stdio loop: `Content-Length: N\r\n\r\n<JSON frame>` both
   directions, protocol on stdout only, logs on stderr. Two exit paths, both load-bearing:
   - `{"op":"shutdown"}` → ack, flush, exit 0 (the clean path);
   - **stdin EOF → exit 0** (the orphan-prevention path: when the test runner dies for any
     reason, the OS closes the pipe and the engine notices).
2. **Host per language** (`hosts/`): Node 22, Python 3.14, JVM 17, Go — each spawns the engine,
   then runs the same scenario list:
   - handshake (incl. the rejection path for an unsupported protocol version);
   - clean shutdown → exit code 0, no process left;
   - stdin-close without shutdown → engine exits (EOF path);
   - **orphan test**: host process is SIGKILLed mid-session from outside; the engine must exit
     within a bounded window;
   - the 1.2 benchmark protocol over the pipe (echo-small ×5000, echo-100k ×500, match-100k ×200,
     same payload file) for direct in-process vs subprocess comparison; spawn-to-ready cold start.
3. **Windows**: no Windows machine in this spike — cross-compile check
   (`x86_64-pc-windows-gnu`), plus a written analysis of the Windows-specific risks (no POSIX
   signals, kill-tree semantics, CRLF/binary-mode stdio). Recorded as analysis, not evidence.

## Layout

```
engine-bin/       pact-engine-toy: LSP-framed stdio server around the toy logic
hosts/node|python|jvm|go   Spawn/shutdown/orphan scenarios + pipe benchmark
FINDINGS.md       Scenario results, pipe-vs-WASM cost, Windows analysis
```
