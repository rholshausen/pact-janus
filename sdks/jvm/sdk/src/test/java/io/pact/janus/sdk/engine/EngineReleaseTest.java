package io.pact.janus.sdk.engine;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * Resolving a released engine (staged plan S0.4 and S0.10, X.1; ADR 0029 decision 3), against a
 * release this test serves itself: what matters is that the pinned digest decides, whatever the
 * server sends. The same cases as the TypeScript suite's {@code test/engine-release.test.ts}.
 */
class EngineReleaseTest {
  private static final byte[] ENGINE =
      "#!/bin/sh\n# not an engine, but these are the bytes that were pinned\n".getBytes(StandardCharsets.UTF_8);
  private static final String DIGEST = sha256(ENGINE);
  private static final String ASSET = EngineRelease.engineAsset();

  /** A release host: serves {@code served} for the one asset of the one version, and counts what it is asked. */
  private static HttpServer server;
  private static String baseUrl;
  private static volatile byte[] served = ENGINE;
  private static final List<String> requests = Collections.synchronizedList(new ArrayList<>());

  @TempDir Path cacheDir;

  @BeforeAll
  static void serve() throws IOException {
    server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
    server.createContext("/", exchange -> {
      String path = exchange.getRequestURI().getPath();
      requests.add(path);
      byte[] body = path.equals("/v1.2.3/" + ASSET) ? served : null;
      exchange.sendResponseHeaders(body != null ? 200 : 404, body != null ? body.length : -1);
      if (body != null) {
        try (OutputStream out = exchange.getResponseBody()) {
          out.write(body);
        }
      }
      exchange.close();
    });
    server.start();
    baseUrl = "http://127.0.0.1:" + server.getAddress().getPort();
  }

  @AfterAll
  static void stop() {
    server.stop(0);
  }

  private static String sha256(byte[] bytes) {
    try {
      return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    } catch (Exception e) {
      throw new IllegalStateException(e);
    }
  }

  private static EngineRelease.Pin pin(String version, Map<String, String> sha256) {
    return new EngineRelease.Pin("example/janus", version, sha256);
  }

  private static EngineRelease.Pin pin() {
    return pin("1.2.3", Map.of(ASSET, DIGEST));
  }

  private static List<Path> list(Path dir) throws IOException {
    try (Stream<Path> files = Files.list(dir)) {
      return files.toList();
    }
  }

  @Test
  @DisplayName("names the engine as the release does, per platform")
  void assetNames() {
    assertEquals("janus-engine-x86_64-unknown-linux-musl", EngineRelease.engineAsset("linux", "x64"));
    assertEquals("janus-engine-aarch64-unknown-linux-musl", EngineRelease.engineAsset("linux", "arm64"));
    assertEquals("janus-engine-aarch64-apple-darwin", EngineRelease.engineAsset("darwin", "arm64"));
    assertEquals("janus-engine-x86_64-apple-darwin", EngineRelease.engineAsset("darwin", "x64"));
    assertEquals("janus-engine-x86_64-pc-windows-msvc.exe", EngineRelease.engineAsset("win32", "x64"));
    IllegalStateException unreleased =
        assertThrows(IllegalStateException.class, () -> EngineRelease.engineAsset("freebsd", "x64"));
    assertTrue(unreleased.getMessage().contains("No janus-engine is released for freebsd/x64"));
  }

  @Test
  @DisplayName("fetches the pinned version's engine once, and keeps it")
  void fetchesOnce() throws Exception {
    served = ENGINE;
    int before = requests.size();

    Path path = EngineRelease.resolve(pin(), cacheDir, baseUrl);

    assertEquals(EngineRelease.enginePath(pin(), cacheDir), path);
    assertArrayEquals(ENGINE, Files.readAllBytes(path));
    assertTrue(Files.isExecutable(path));
    assertEquals(path, EngineRelease.resolve(pin(), cacheDir, baseUrl));
    assertEquals(List.of("/v1.2.3/" + ASSET), List.copyOf(requests.subList(before, requests.size())));
  }

  @Test
  @DisplayName("refuses a download that is not what was pinned, and keeps nothing of it")
  void refusesAnotherDigest() throws Exception {
    served = "#!/bin/sh\n# something else\n".getBytes(StandardCharsets.UTF_8);

    IOException refused = assertThrows(IOException.class, () -> EngineRelease.resolve(pin(), cacheDir, baseUrl));

    assertTrue(refused.getMessage().contains("is not the janus-engine that was pinned"), refused.getMessage());
    assertTrue(refused.getMessage().contains("the pin says " + DIGEST), refused.getMessage());
    assertEquals(List.of(), list(cacheDir));
  }

  @Test
  @DisplayName("fetches again when what is in the cache is not what was pinned")
  void refetchesWhenTheCacheDiffers() throws Exception {
    served = ENGINE;
    Path path = EngineRelease.resolve(pin(), cacheDir, baseUrl);
    byte[] other = "tampered with since".getBytes(StandardCharsets.UTF_8);
    EngineRelease.Pin moved = pin("1.2.3", Map.of(ASSET, sha256(other)));
    served = other;

    assertEquals(path, EngineRelease.resolve(moved, cacheDir, baseUrl));
    assertArrayEquals(other, Files.readAllBytes(path));
    assertEquals(List.of(path), list(path.getParent()));
  }

  @Test
  @DisplayName("says so when nothing is pinned, or nothing for this platform")
  void nothingPinned() {
    IllegalStateException unpinned = assertThrows(IllegalStateException.class,
        () -> EngineRelease.resolve(pin(null, Map.of(ASSET, DIGEST)), cacheDir, baseUrl));
    assertTrue(unpinned.getMessage().contains("No janus-engine release is pinned yet"));
    IllegalStateException noDigest = assertThrows(IllegalStateException.class,
        () -> EngineRelease.resolve(pin("1.2.3", Map.of()), cacheDir, baseUrl));
    assertTrue(noDigest.getMessage().contains("pinned with no digest for " + ASSET));
  }

  @Test
  @DisplayName("says where it looked when the release does not have the file")
  void notInTheRelease() {
    IOException missing = assertThrows(IOException.class,
        () -> EngineRelease.resolve(pin("9.9.9", Map.of(ASSET, DIGEST)), cacheDir, baseUrl));
    assertTrue(missing.getMessage().contains("could not be fetched from " + baseUrl + "/v9.9.9/"), missing.getMessage());
    assertTrue(missing.getMessage().contains("HTTP 404"), missing.getMessage());
  }

  @Test
  @DisplayName("reads the pin every SDK in this repository shares")
  void readsTheSharedPin() throws Exception {
    Path file = cacheDir.resolve("engine-release.json");
    Files.writeString(file, """
        {"$comment": "ignored", "repository": "example/janus", "version": "1.2.3",
         "sha256": {"%s": "%s"}}
        """.formatted(ASSET, DIGEST));
    assertEquals(pin(), EngineRelease.Pin.read(file));
  }
}
