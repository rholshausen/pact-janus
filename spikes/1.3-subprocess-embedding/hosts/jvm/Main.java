// JVM host leg: same scenarios as hosts/node/main.mjs. No dependencies —
// run with Java 17 single-file launch:
//   java Main.java            all scenarios
//   java Main.java --runner   orphan-test helper
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.function.DoubleFunction;

public final class Main {

    static final String ENGINE = "../../engine-bin/target/release/pact-engine-toy";

    static final class Client {
        final Process proc;
        final OutputStream stdin;
        final InputStream stdout;

        Client() throws IOException {
            proc = new ProcessBuilder(ENGINE).redirectError(ProcessBuilder.Redirect.DISCARD).start();
            stdin = proc.getOutputStream();
            stdout = new BufferedInputStream(proc.getInputStream());
        }

        String request(String frame) throws IOException {
            byte[] body = frame.getBytes(StandardCharsets.UTF_8);
            stdin.write(("Content-Length: " + body.length + "\r\n\r\n").getBytes(StandardCharsets.UTF_8));
            stdin.write(body);
            stdin.flush();
            int contentLength = -1;
            StringBuilder line = new StringBuilder();
            while (true) {
                int b = stdout.read();
                if (b < 0) throw new EOFException("engine closed stdout");
                if (b == '\n') {
                    if (line.length() == 0) break; // blank line: headers done
                    String h = line.toString();
                    if (h.startsWith("Content-Length:")) {
                        contentLength = Integer.parseInt(h.substring(15).trim());
                    }
                    line.setLength(0);
                } else if (b != '\r') {
                    line.append((char) b);
                }
            }
            byte[] resp = stdout.readNBytes(contentLength);
            return new String(resp, StandardCharsets.UTF_8);
        }
    }

    static void ok(boolean cond, String msg) {
        if (!cond) throw new AssertionError("FAIL: " + msg);
        System.out.println("  ok: " + msg);
    }

    static boolean pidAlive(long pid) {
        return ProcessHandle.of(pid).map(ProcessHandle::isAlive).orElse(false);
    }

    static void stats(String name, long[] samples) {
        long[] s = samples.clone();
        Arrays.sort(s);
        DoubleFunction<Double> at = q -> s[Math.min(s.length - 1, (int) (q * s.length))] / 1e3;
        System.out.printf("  %s: median %.1fµs  p95 %.1fµs  min %.1fµs%n", name, at.apply(0.5), at.apply(0.95), s[0] / 1e3);
    }

    static void bench(Client c, String name, String frame, int warmup, int iters) throws IOException {
        for (int i = 0; i < warmup; i++) c.request(frame);
        long[] samples = new long[iters];
        for (int i = 0; i < iters; i++) {
            long t0 = System.nanoTime();
            c.request(frame);
            samples[i] = System.nanoTime() - t0;
        }
        stats(name, samples);
    }

    public static void main(String[] args) throws Exception {
        String doc = Files.readString(Path.of("../../../1.2-wasm-embedding/payloads/order-100kb.json"));
        Map<String, String> frames = new LinkedHashMap<>();
        frames.put("handshake", "{\"op\":\"handshake\",\"protocol-versions\":[1]}");
        frames.put("handshake-bad", "{\"op\":\"handshake\",\"protocol-versions\":[99]}");
        frames.put("echo-small", "{\"op\":\"echo\",\"payload\":{\"ping\":1}}");
        frames.put("echo-100k", "{\"op\":\"echo\",\"payload\":" + doc + "}");
        frames.put("match-100k", "{\"op\":\"match-type\",\"expected\":" + doc + ",\"actual\":" + doc + "}");
        frames.put("shutdown", "{\"op\":\"shutdown\"}");

        if (args.length > 0 && args[0].equals("--runner")) {
            Client c = new Client();
            c.request(frames.get("handshake"));
            System.out.println(c.proc.pid());
            System.out.flush();
            Thread.sleep(60_000);
            System.exit(1);
        }

        System.out.println("scenario: handshake");
        Client c = new Client();
        String bad = c.request(frames.get("handshake-bad"));
        ok(bad.contains("protocol-version-unsupported"), "unsupported version rejected: " + bad);
        String good = c.request(frames.get("handshake"));
        ok(good.contains("\"protocol-version\":1"), "version 1 negotiated");
        c.proc.destroyForcibly().waitFor();

        System.out.println("scenario: spawn-to-ready");
        long[] ready = new long[10];
        for (int i = 0; i < ready.length; i++) {
            long t0 = System.nanoTime();
            c = new Client();
            c.request(frames.get("handshake"));
            ready[i] = System.nanoTime() - t0;
            c.stdin.close();
            c.proc.waitFor();
        }
        stats("spawn-to-ready", ready);

        System.out.println("scenario: pipe benchmark");
        c = new Client();
        c.request(frames.get("handshake"));
        bench(c, "echo-small", frames.get("echo-small"), 500, 5000);
        bench(c, "echo-100k", frames.get("echo-100k"), 50, 500);
        bench(c, "match-100k", frames.get("match-100k"), 20, 200);
        c.stdin.close();
        c.proc.waitFor();

        System.out.println("scenario: clean shutdown");
        c = new Client();
        c.request(frames.get("handshake"));
        String ack = c.request(frames.get("shutdown"));
        ok(ack.contains("\"shutting-down\":true"), "shutdown acknowledged");
        int code = c.proc.waitFor();
        ok(code == 0, "exit code 0 (got " + code + ")");
        ok(!pidAlive(c.proc.pid()), "no process left");

        System.out.println("scenario: EOF exit");
        c = new Client();
        c.request(frames.get("handshake"));
        c.stdin.close();
        boolean exited = c.proc.waitFor(2, java.util.concurrent.TimeUnit.SECONDS);
        ok(exited && c.proc.exitValue() == 0, "engine exited on EOF with code 0");

        System.out.println("scenario: orphan on runner SIGKILL");
        String javaBin = ProcessHandle.current().info().command().orElse("java");
        Process runner = new ProcessBuilder(javaBin, "Main.java", "--runner")
                .redirectError(ProcessBuilder.Redirect.DISCARD).start();
        BufferedReader r = new BufferedReader(new InputStreamReader(runner.getInputStream()));
        long enginePid = Long.parseLong(r.readLine().trim());
        ok(pidAlive(enginePid), "engine (pid " + enginePid + ") alive under runner (pid " + runner.pid() + ")");
        runner.destroyForcibly(); // SIGKILL on Linux
        int waited = 0;
        while (pidAlive(enginePid) && waited < 5000) {
            Thread.sleep(50);
            waited += 50;
        }
        ok(!pidAlive(enginePid), "engine exited within " + waited + "ms of runner SIGKILL");
        runner.waitFor();

        System.out.println("all scenarios passed");
    }
}
