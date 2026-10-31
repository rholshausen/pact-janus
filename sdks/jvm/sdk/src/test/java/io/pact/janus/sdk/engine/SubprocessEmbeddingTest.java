package io.pact.janus.sdk.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeFalse;

import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * When the SDK cannot start {@code janus-engine}, the error says why and what to do, laid out for a
 * reader: someone moving from pact-jvm has no checkout to build one in. Told nothing, the SDK fetches
 * the engine it was released with (ADR 0060 decision 6), and a fetch that fails is laid out the same
 * way.
 */
class SubprocessEmbeddingTest {
  private static final IOException MISSING = new IOException("Cannot run program \"janus-engine\": error=2");

  @Test
  void saysWhyThenEachWayToGetOneUnwrappedWithNoWidth() {
    assertEquals(
        String.join("\n",
            "JANUS_ENGINE names /opt/janus-engine, which does not exist.",
            "",
            "The SDK runs the Janus engine as its own executable. Either:",
            "  - unset JANUS_ENGINE, and the SDK fetches the janus-engine it was released with; or",
            "  - download janus-engine-x86_64-unknown-linux-musl from " + SubprocessEmbedding.RELEASES + "/tag/v"
                + EngineRelease.PIN.version() + ", make it executable, and set JANUS_ENGINE to its path; or",
            "  - in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin janus-engine` and set "
                + "JANUS_ENGINE to target/debug/janus-engine."),
        SubprocessEmbedding.notStarted("/opt/janus-engine", "JANUS_ENGINE", MISSING, "linux", "x64", null));
  }

  @Test
  void isWrappedAtSpacesOnlyUnderItsBullet() {
    List<String> lines =
        SubprocessEmbedding.notStarted("/opt/janus-engine", "JANUS_ENGINE", MISSING, "linux", "x64", 60).lines().toList();
    for (String line : lines) {
      assertTrue(line.length() <= 60 || line.contains("`") || line.contains(SubprocessEmbedding.RELEASES), line);
    }
    assertTrue(lines.stream().anyMatch(line -> line.startsWith("    " + SubprocessEmbedding.RELEASES + "/tag/v")),
        "the URL is whole");
    assertTrue(lines.stream().anyMatch(line -> line.contains("`cargo build -p pact_janus_cli --bin janus-engine`")),
        "the command is whole");
    assertEquals(3, lines.stream().filter(line -> line.startsWith("  - ")).count());
  }

  @Test
  void aPathThatDoesNotExistIsNamedWithWhatPointedAtIt(@TempDir Path dir) {
    String missing = dir.resolve("janus-engine").toString();
    String message = SubprocessEmbedding.notStarted(missing, "SubprocessEmbedding.of", MISSING, "darwin", "arm64", null);
    assertTrue(message.startsWith("SubprocessEmbedding.of names " + missing + ", which does not exist.\n"), message);
    assertTrue(message.contains("leave out SubprocessEmbedding.of, and the SDK fetches"), message);
    assertTrue(message.contains("janus-engine-aarch64-apple-darwin from"), message);
  }

  @Test
  void aPlatformWithNoReleaseIsToldToBuildOne() {
    assertEquals(
        String.join("\n",
            "No janus-engine is released for freebsd/x64.",
            "",
            "The SDK fetches the Janus engine it was released with when it first runs. To get one:",
            "  - in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin janus-engine` and set "
                + "JANUS_ENGINE to target/debug/janus-engine. No janus-engine is released for freebsd/x64."),
        SubprocessEmbedding.notFetched("No janus-engine is released for freebsd/x64", "freebsd", "x64", null));
  }

  @Test
  void aReleaseThatCannotBeFetchedSaysToNameAMirrorOrFetchItByHand() {
    List<String> lines = SubprocessEmbedding.notFetched(
        "janus-engine 0.4.0 could not be fetched from https://example.com: HTTP 404", "linux", "arm64", null)
        .lines().toList();
    assertEquals(List.of(
        "janus-engine 0.4.0 could not be fetched from https://example.com: HTTP 404.",
        "",
        "The SDK fetches the Janus engine it was released with when it first runs. Either:",
        "  - set JANUS_ENGINE_MIRROR to a server holding janus-engine's release files, as "
            + "`<mirror>/v<version>/<file>`; or"), lines.subList(0, 4));
    assertTrue(lines.get(4).contains("download janus-engine-aarch64-unknown-linux-musl from "), lines.get(4));
  }

  @Test
  void namedByNothingItIsThePinnedReleaseAndAFetchThatFailsIsThatError(@TempDir Path cache) throws IOException {
    HttpServer server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
    server.createContext("/", exchange -> {
      exchange.sendResponseHeaders(404, -1);
      exchange.close();
    });
    server.start();
    try {
      String mirror = "http://127.0.0.1:" + server.getAddress().getPort();
      SubprocessEmbedding embedding = SubprocessEmbedding.fromEnvironment(
          Map.of("JANUS_ENGINE_MIRROR", mirror, "JANUS_ENGINE_CACHE", cache.toString()));
      assertEquals(List.of(), embedding.command());
      EngineNotStartedException failed = assertThrows(EngineNotStartedException.class, embedding::open);
      String version = EngineRelease.PIN.version();
      assertTrue(failed.getMessage().startsWith("janus-engine " + version + " could not be fetched from " + mirror
          + "/v" + version + "/janus-engine-"), failed.getMessage());
      assertTrue(failed.getMessage().contains(": HTTP 404.\n"), failed.getMessage());
    } finally {
      server.stop(0);
    }
  }

  @Test
  void startingAMissingOrNonExecutableEngineThrowsItWithNoStackOrCause(@TempDir Path dir) throws IOException {
    Path engine = dir.resolve("janus-engine");
    EngineNotStartedException missing =
        assertThrows(EngineNotStartedException.class, () -> SubprocessEmbedding.of(engine).open());
    assertTrue(missing.getMessage().startsWith("SubprocessEmbedding.of names " + engine + ", which does not exist."),
        missing.getMessage());
    assertEquals(0, missing.getStackTrace().length);
    assertEquals(null, missing.getCause());
    assumeFalse(SubprocessEmbedding.platform().equals("win32"), "Windows has no executable bit");
    Files.writeString(engine, "not an executable");
    IOException notExecutable = assertThrows(IOException.class, () -> SubprocessEmbedding.of(engine).open());
    assertTrue(notExecutable.getMessage().contains(engine + " is not executable (chmod +x it)."),
        notExecutable.getMessage());
  }
}
