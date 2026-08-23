# Python host leg: wasmtime-py 48.
# Path A: the real component via wasmtime.component (dynamic API, add_wasip2 for WASI).
# Path B: core module + hand-built C-ABI shim via the classic API.
import json
import statistics
import time
from pathlib import Path

import wasmtime
import wasmtime.component as wc

HERE = Path(__file__).parent
COMPONENT = (HERE / "../../engine-toy/target/wasm32-wasip1/release/toy_engine_component.wasm").resolve()
CORE = (HERE / "../../engine-toy/target/wasm32-unknown-unknown/release/toy_engine_core.wasm").resolve()
DOC = (HERE / "../../payloads/order-100kb.json").resolve().read_text()

FRAMES = {
    "handshake": b'{"op":"handshake","protocol-versions":[1]}',
    "echo-small": b'{"op":"echo","payload":{"ping":1}}',
    "echo-100k": ('{"op":"echo","payload":%s}' % DOC).encode(),
    "match-100k": ('{"op":"match-type","expected":%s,"actual":%s}' % (DOC, DOC)).encode(),
}


def report(name, samples_ns):
    s = sorted(samples_ns)
    at = lambda q: s[min(len(s) - 1, int(q * len(s)))] / 1e3
    print(f"{name}: median {at(0.5):.1f}µs  p95 {at(0.95):.1f}µs  min {s[0]/1e3:.1f}µs")


def bench(name, fn, warmup, iters):
    for _ in range(warmup):
        fn()
    samples = []
    for _ in range(iters):
        t0 = time.perf_counter_ns()
        fn()
        samples.append(time.perf_counter_ns() - t0)
    report(name, samples)


def check(name, resp_bytes, expect_key="ok"):
    resp = json.loads(bytes(resp_bytes))
    assert expect_key in resp, f"{name}: unexpected response {str(resp)[:200]}"


def run_leg(title, make_pipe):
    print(f"--- {title}")
    cold, pipe = [], None
    for _ in range(20):
        t0 = time.perf_counter_ns()
        pipe = make_pipe()
        cold.append(time.perf_counter_ns() - t0)
    print(f"cold-start: first {cold[0]/1e6:.1f}ms  median {statistics.median(cold)/1e6:.1f}ms")
    check("handshake", pipe(FRAMES["handshake"]))
    check("echo-small", pipe(FRAMES["echo-small"]))
    check("match-100k", pipe(FRAMES["match-100k"]))
    bench("echo-small", lambda: pipe(FRAMES["echo-small"]), 500, 5000)
    bench("echo-100k", lambda: pipe(FRAMES["echo-100k"]), 50, 500)
    bench("match-100k", lambda: pipe(FRAMES["match-100k"]), 20, 200)


# --- Path A: component model
component_bytes = COMPONENT.read_bytes()


def make_component_pipe():
    engine = wasmtime.Engine()
    store = wasmtime.Store(engine)
    component = wc.Component(engine, component_bytes)
    linker = wc.Linker(engine)
    linker.add_wasip2()
    store.set_wasi(wasmtime.WasiConfig())
    instance = linker.instantiate(store, component)
    idx = instance.get_export_index(store, "pact:toy-engine/pipe@0.1.0")
    fidx = instance.get_export_index(store, "call", idx)
    func = instance.get_func(store, fidx)
    return lambda req: func(store, req)


# --- Path B: core + shim
core_bytes = CORE.read_bytes()


def make_core_pipe():
    engine = wasmtime.Engine()
    store = wasmtime.Store(engine)
    module = wasmtime.Module(engine, core_bytes)
    instance = wasmtime.Instance(store, module, [])
    exports = instance.exports(store)
    memory, alloc, dealloc, call = exports["memory"], exports["alloc"], exports["dealloc"], exports["call"]

    def pipe(request):
        req_len = len(request)
        req_ptr = alloc(store, req_len)
        memory.write(store, request, req_ptr)
        packed = call(store, req_ptr, req_len)
        resp_ptr, resp_len = (packed >> 32) & 0xFFFFFFFF, packed & 0xFFFFFFFF
        response = memory.read(store, resp_ptr, resp_ptr + resp_len)
        dealloc(store, resp_ptr, resp_len)
        dealloc(store, req_ptr, req_len)
        return response

    return pipe


if __name__ == "__main__":
    run_leg("component (wasmtime.component, dynamic)", make_component_pipe)
    run_leg("core + C-ABI shim", make_core_pipe)
