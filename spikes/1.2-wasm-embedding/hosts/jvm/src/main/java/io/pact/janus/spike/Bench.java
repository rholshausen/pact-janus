package io.pact.janus.spike;

import com.dylibso.chicory.compiler.MachineFactoryCompiler;
import com.dylibso.chicory.runtime.ExportFunction;
import com.dylibso.chicory.runtime.Instance;
import com.dylibso.chicory.wasm.Parser;
import com.dylibso.chicory.wasm.WasmModule;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.function.Function;
import java.util.function.Supplier;

/**
 * JVM host leg: Chicory 1.4 (pure-JVM runtime, zero native deps).
 * Attempts the component binary (recording the failure), then benchmarks the
 * core module + C-ABI shim under both the interpreter and the runtime
 * bytecode compiler backends.
 */
public final class Bench {

    interface Pipe {
        byte[] call(byte[] request);
    }

    static Pipe makePipe(Instance instance) {
        ExportFunction alloc = instance.export("alloc");
        ExportFunction dealloc = instance.export("dealloc");
        ExportFunction call = instance.export("call");
        return request -> {
            int reqLen = request.length;
            int reqPtr = (int) alloc.apply(reqLen)[0];
            instance.memory().write(reqPtr, request);
            long packed = call.apply(reqPtr, reqLen)[0];
            int respPtr = (int) (packed >>> 32);
            int respLen = (int) (packed & 0xffffffffL);
            byte[] response = instance.memory().readBytes(respPtr, respLen);
            dealloc.apply(respPtr, respLen);
            dealloc.apply(reqPtr, reqLen);
            return response;
        };
    }

    static void stats(String name, long[] samples) {
        long[] s = samples.clone();
        Arrays.sort(s);
        Function<Double, Double> at = q -> s[Math.min(s.length - 1, (int) (q * s.length))] / 1e3;
        System.out.printf("%s: median %.1fµs  p95 %.1fµs  min %.1fµs%n",
                name, at.apply(0.5), at.apply(0.95), s[0] / 1e3);
    }

    static void bench(String name, Runnable fn, int warmup, int iters) {
        for (int i = 0; i < warmup; i++) fn.run();
        long[] samples = new long[iters];
        for (int i = 0; i < iters; i++) {
            long t0 = System.nanoTime();
            fn.run();
            samples[i] = System.nanoTime() - t0;
        }
        stats(name, samples);
    }

    static void check(String name, byte[] resp) {
        String s = new String(resp, StandardCharsets.UTF_8);
        if (!s.startsWith("{\"ok\"")) {
            throw new IllegalStateException(name + ": unexpected response " + s.substring(0, Math.min(200, s.length())));
        }
    }

    static void runLeg(String title, Supplier<Pipe> makeFresh, Map<String, byte[]> frames) {
        System.out.println("--- " + title);
        long[] cold = new long[20];
        Pipe pipe = null;
        for (int i = 0; i < cold.length; i++) {
            long t0 = System.nanoTime();
            pipe = makeFresh.get();
            cold[i] = System.nanoTime() - t0;
        }
        System.out.printf("cold-start: first %.1fms%n", cold[0] / 1e6);
        stats("cold-start (per-iter)", cold);

        check("handshake", pipe.call(frames.get("handshake")));
        check("echo-small", pipe.call(frames.get("echo-small")));
        check("match-100k", pipe.call(frames.get("match-100k")));

        Pipe p = pipe;
        bench("echo-small", () -> p.call(frames.get("echo-small")), 500, 5000);
        bench("echo-100k", () -> p.call(frames.get("echo-100k")), 50, 500);
        bench("match-100k", () -> p.call(frames.get("match-100k")), 20, 200);
    }

    public static void main(String[] args) throws Exception {
        Path base = Path.of("../..").toAbsolutePath().normalize();
        byte[] doc = Files.readAllBytes(base.resolve("payloads/order-100kb.json"));
        String docStr = new String(doc, StandardCharsets.UTF_8);

        Map<String, byte[]> frames = new LinkedHashMap<>();
        frames.put("handshake", "{\"op\":\"handshake\",\"protocol-versions\":[1]}".getBytes(StandardCharsets.UTF_8));
        frames.put("echo-small", "{\"op\":\"echo\",\"payload\":{\"ping\":1}}".getBytes(StandardCharsets.UTF_8));
        frames.put("echo-100k", ("{\"op\":\"echo\",\"payload\":" + docStr + "}").getBytes(StandardCharsets.UTF_8));
        frames.put("match-100k", ("{\"op\":\"match-type\",\"expected\":" + docStr + ",\"actual\":" + docStr + "}")
                .getBytes(StandardCharsets.UTF_8));

        // Attempt 1: the component binary — record what Chicory does with it.
        try {
            Parser.parse(base.resolve("engine-toy/target/wasm32-wasip1/release/toy_engine_component.wasm"));
            System.out.println("component binary: parsed (unexpected!)");
        } catch (Exception e) {
            System.out.println("component binary: " + e.getClass().getSimpleName() + ": " + e.getMessage());
        }

        byte[] coreBytes = Files.readAllBytes(
                base.resolve("engine-toy/target/wasm32-unknown-unknown/release/toy_engine_core.wasm"));

        runLeg("interpreter", () -> {
            WasmModule module = Parser.parse(coreBytes);
            return makePipe(Instance.builder(module).build());
        }, frames);

        runLeg("bytecode compiler", () -> {
            WasmModule module = Parser.parse(coreBytes);
            return makePipe(Instance.builder(module)
                    .withMachineFactory(MachineFactoryCompiler::compile)
                    .build());
        }, frames);
    }
}
