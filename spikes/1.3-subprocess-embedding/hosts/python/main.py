# Python host leg: same scenarios as hosts/node/main.mjs.
# `python3 main.py` — all scenarios; `python3 main.py --runner` — orphan helper.
import json
import os
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).parent
ENGINE = str((HERE / "../../engine-bin/target/release/pact-engine-toy").resolve())
DOC = (HERE / "../../../1.2-wasm-embedding/payloads/order-100kb.json").resolve().read_text()

FRAMES = {
    "handshake": '{"op":"handshake","protocol-versions":[1]}',
    "handshake-bad": '{"op":"handshake","protocol-versions":[99]}',
    "echo-small": '{"op":"echo","payload":{"ping":1}}',
    "echo-100k": '{"op":"echo","payload":%s}' % DOC,
    "match-100k": '{"op":"match-type","expected":%s,"actual":%s}' % (DOC, DOC),
    "shutdown": '{"op":"shutdown"}',
}


class EngineClient:
    def __init__(self):
        self.proc = subprocess.Popen(
            [ENGINE], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL
        )

    def request(self, frame: str) -> dict:
        body = frame.encode()
        self.proc.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body))
        self.proc.stdin.write(body)
        self.proc.stdin.flush()
        headers = b""
        while not headers.endswith(b"\r\n\r\n"):
            b1 = self.proc.stdout.read(1)
            if not b1:
                raise EOFError("engine closed stdout")
            headers += b1
        m = re.search(rb"Content-Length:\s*(\d+)", headers)
        return json.loads(self.proc.stdout.read(int(m.group(1))))


def ok(cond, msg):
    if not cond:
        raise AssertionError(f"FAIL: {msg}")
    print(f"  ok: {msg}")


def pid_alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except OSError:
        return False


def stats(name, samples_ns):
    s = sorted(samples_ns)
    at = lambda q: s[min(len(s) - 1, int(q * len(s)))] / 1e3
    print(f"  {name}: median {at(0.5):.1f}µs  p95 {at(0.95):.1f}µs  min {s[0]/1e3:.1f}µs")


def bench(client, name, frame, warmup, iters):
    for _ in range(warmup):
        client.request(frame)
    samples = []
    for _ in range(iters):
        t0 = time.perf_counter_ns()
        client.request(frame)
        samples.append(time.perf_counter_ns() - t0)
    stats(name, samples)


if "--runner" in sys.argv:
    c = EngineClient()
    c.request(FRAMES["handshake"])
    print(c.proc.pid, flush=True)
    time.sleep(60)
    sys.exit(1)

print("scenario: handshake")
c = EngineClient()
bad = c.request(FRAMES["handshake-bad"])
ok(bad.get("err", {}).get("code") == "protocol-version-unsupported", f"unsupported version rejected: {bad['err']}")
good = c.request(FRAMES["handshake"])
ok(good.get("ok", {}).get("protocol-version") == 1, "version 1 negotiated")
c.proc.kill()
c.proc.wait()

print("scenario: spawn-to-ready")
samples = []
for _ in range(10):
    t0 = time.perf_counter_ns()
    c = EngineClient()
    c.request(FRAMES["handshake"])
    samples.append(time.perf_counter_ns() - t0)
    c.proc.stdin.close()
    c.proc.wait()
stats("spawn-to-ready", samples)

print("scenario: pipe benchmark")
c = EngineClient()
c.request(FRAMES["handshake"])
bench(c, "echo-small", FRAMES["echo-small"], 500, 5000)
bench(c, "echo-100k", FRAMES["echo-100k"], 50, 500)
bench(c, "match-100k", FRAMES["match-100k"], 20, 200)
c.proc.stdin.close()
c.proc.wait()

print("scenario: clean shutdown")
c = EngineClient()
c.request(FRAMES["handshake"])
ack = c.request(FRAMES["shutdown"])
ok(ack.get("ok", {}).get("shutting-down") is True, "shutdown acknowledged")
code = c.proc.wait(timeout=2)
ok(code == 0, f"exit code 0 (got {code})")
ok(not pid_alive(c.proc.pid), "no process left")

print("scenario: EOF exit")
c = EngineClient()
c.request(FRAMES["handshake"])
c.proc.stdin.close()
code = c.proc.wait(timeout=2)
ok(code == 0, f"engine exited on EOF with code 0 (got {code})")

print("scenario: orphan on runner SIGKILL")
runner = subprocess.Popen([sys.executable, __file__, "--runner"], stdout=subprocess.PIPE)
engine_pid = int(runner.stdout.readline().strip())
ok(pid_alive(engine_pid), f"engine (pid {engine_pid}) alive under runner (pid {runner.pid})")
os.kill(runner.pid, signal.SIGKILL)
waited = 0
while pid_alive(engine_pid) and waited < 5000:
    time.sleep(0.05)
    waited += 50
ok(not pid_alive(engine_pid), f"engine exited within {waited}ms of runner SIGKILL")
runner.wait()

print("all scenarios passed")
