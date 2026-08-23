// Go host leg: wazero (pure-Go runtime, no cgo). Core WASM + hand-built C-ABI
// shim; also attempts to load the component binary to record what happens.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"time"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

var frames = map[string][]byte{}

func loadFrames() {
	doc, err := os.ReadFile("../../payloads/order-100kb.json")
	if err != nil {
		panic(err)
	}
	frames["handshake"] = []byte(`{"op":"handshake","protocol-versions":[1]}`)
	frames["echo-small"] = []byte(`{"op":"echo","payload":{"ping":1}}`)
	frames["echo-100k"] = append(append([]byte(`{"op":"echo","payload":`), doc...), '}')
	m := append([]byte(`{"op":"match-type","expected":`), doc...)
	m = append(m, []byte(`,"actual":`)...)
	m = append(m, doc...)
	frames["match-100k"] = append(m, '}')
}

type pipe struct {
	ctx     context.Context
	mod     api.Module
	alloc   api.Function
	dealloc api.Function
	call    api.Function
}

func makePipe(ctx context.Context, mod api.Module) *pipe {
	return &pipe{ctx, mod, mod.ExportedFunction("alloc"), mod.ExportedFunction("dealloc"), mod.ExportedFunction("call")}
}

func (p *pipe) Call(request []byte) []byte {
	reqLen := uint64(len(request))
	res, err := p.alloc.Call(p.ctx, reqLen)
	if err != nil {
		panic(err)
	}
	reqPtr := res[0]
	if !p.mod.Memory().Write(uint32(reqPtr), request) {
		panic("write out of range")
	}
	res, err = p.call.Call(p.ctx, reqPtr, reqLen)
	if err != nil {
		panic(err)
	}
	respPtr, respLen := uint32(res[0]>>32), uint32(res[0]&0xffffffff)
	view, ok := p.mod.Memory().Read(respPtr, respLen)
	if !ok {
		panic("read out of range")
	}
	response := append([]byte(nil), view...)
	p.dealloc.Call(p.ctx, uint64(respPtr), uint64(respLen))
	p.dealloc.Call(p.ctx, reqPtr, reqLen)
	return response
}

func stats(name string, samples []time.Duration) {
	sort.Slice(samples, func(i, j int) bool { return samples[i] < samples[j] })
	at := func(q float64) time.Duration { return samples[int(q*float64(len(samples)-1))] }
	fmt.Printf("%s: median %.1fµs  p95 %.1fµs  min %.1fµs\n", name,
		float64(at(0.5).Nanoseconds())/1e3, float64(at(0.95).Nanoseconds())/1e3, float64(samples[0].Nanoseconds())/1e3)
}

func bench(name string, warmup, iters int, fn func()) {
	for i := 0; i < warmup; i++ {
		fn()
	}
	samples := make([]time.Duration, iters)
	for i := 0; i < iters; i++ {
		t0 := time.Now()
		fn()
		samples[i] = time.Since(t0)
	}
	stats(name, samples)
}

func checkResponse(name string, bytes []byte, expectKey string) {
	var resp map[string]json.RawMessage
	if err := json.Unmarshal(bytes, &resp); err != nil {
		panic(name + ": " + err.Error())
	}
	if _, ok := resp[expectKey]; !ok {
		panic(fmt.Sprintf("%s: unexpected response %.200s", name, bytes))
	}
}

func main() {
	loadFrames()
	ctx := context.Background()

	// Attempt 1: the component binary — expected to be rejected (record the error).
	componentBytes, err := os.ReadFile("../../engine-toy/target/wasm32-wasip1/release/toy_engine_component.wasm")
	if err != nil {
		panic(err)
	}
	r := wazero.NewRuntime(ctx)
	_, err = r.Instantiate(ctx, componentBytes)
	fmt.Printf("component binary: %v\n", err)
	r.Close(ctx)

	// Core module + shim.
	coreBytes, err := os.ReadFile("../../engine-toy/target/wasm32-unknown-unknown/release/toy_engine_core.wasm")
	if err != nil {
		panic(err)
	}

	cold := make([]time.Duration, 20)
	var p *pipe
	var runtime wazero.Runtime
	for i := range cold {
		if runtime != nil {
			runtime.Close(ctx)
		}
		t0 := time.Now()
		runtime = wazero.NewRuntime(ctx) // compiler backend on amd64
		mod, err := runtime.Instantiate(ctx, coreBytes)
		if err != nil {
			panic(err)
		}
		p = makePipe(ctx, mod)
		cold[i] = time.Since(t0)
	}
	first := cold[0]
	stats("cold-start (per-iter)", cold)
	fmt.Printf("cold-start: first %.1fms\n", float64(first.Nanoseconds())/1e6)

	// Same, with a shared CompilationCache (what a real SDK would do).
	cache := wazero.NewCompilationCache()
	cached := make([]time.Duration, 20)
	for i := range cached {
		t0 := time.Now()
		rt := wazero.NewRuntimeWithConfig(ctx, wazero.NewRuntimeConfig().WithCompilationCache(cache))
		if _, err := rt.Instantiate(ctx, coreBytes); err != nil {
			panic(err)
		}
		cached[i] = time.Since(t0)
		rt.Close(ctx)
	}
	stats("cold-start (shared cache)", cached)

	// One runtime, module precompiled once, fresh instance per iteration.
	rt := wazero.NewRuntime(ctx)
	defer rt.Close(ctx)
	compiled, err := rt.CompileModule(ctx, coreBytes)
	if err != nil {
		panic(err)
	}
	inst := make([]time.Duration, 20)
	for i := range inst {
		t0 := time.Now()
		mod, err := rt.InstantiateModule(ctx, compiled, wazero.NewModuleConfig().WithName(fmt.Sprintf("i%d", i)))
		if err != nil {
			panic(err)
		}
		inst[i] = time.Since(t0)
		mod.Close(ctx)
	}
	stats("instantiate (precompiled)", inst)

	checkResponse("handshake", p.Call(frames["handshake"]), "ok")
	checkResponse("echo-small", p.Call(frames["echo-small"]), "ok")
	checkResponse("match-100k", p.Call(frames["match-100k"]), "ok")

	bench("echo-small", 500, 5000, func() { p.Call(frames["echo-small"]) })
	bench("echo-100k", 50, 500, func() { p.Call(frames["echo-100k"]) })
	bench("match-100k", 20, 200, func() { p.Call(frames["match-100k"]) })
}
