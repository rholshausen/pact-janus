package io.pact.janus.sdk.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeFalse;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * When the SDK cannot start {@code janus-engine}, the error says why and what to do, laid out for a
 * reader: someone moving from pact-jvm has no checkout to build one in.
 */
class SubprocessEmbeddingTest {
  private static final IOException MISSING = new IOException("Cannot run program \"janus-engine\": error=2");

  @Test
  void saysWhyThenEachWayToGetOneUnwrappedWithNoWidth() {
    assertEquals(
        String.join("\n",
            "there is no janus-engine on the PATH, and JANUS_ENGINE is not set.",
            "",
            "The SDK runs the Janus engine as its own executable, and does not ship it. Either:",
            "  - download janus-engine-x86_64-unknown-linux-musl from " + SubprocessEmbedding.RELEASES
                + ", make it executable, and put it on the PATH as janus-engine or set JANUS_ENGINE to its path; or",
            "  - in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin janus-engine` and set "
                + "JANUS_ENGINE to target/debug/janus-engine."),
        SubprocessEmbedding.notStarted("janus-engine", "PATH", MISSING, "linux", "x64", null));
  }

  @Test
  void isWrappedAtSpacesOnlyUnderItsBullet() {
    List<String> lines = SubprocessEmbedding.notStarted("janus-engine", "PATH", MISSING, "linux", "x64", 60).lines().toList();
    for (String line : lines) {
      assertTrue(line.length() <= 60 || line.contains("`"), line);
    }
    assertTrue(lines.stream().anyMatch(line -> line.startsWith("    " + SubprocessEmbedding.RELEASES + ",")), "the URL is whole");
    assertTrue(lines.stream().anyMatch(line -> line.contains("`cargo build -p pact_janus_cli --bin janus-engine`")),
        "the command is whole");
    assertEquals(2, lines.stream().filter(line -> line.startsWith("  - ")).count());
  }

  @Test
  void aPathThatDoesNotExistIsNamedWithWhatPointedAtIt(@TempDir Path dir) {
    String missing = dir.resolve("janus-engine").toString();
    String message = SubprocessEmbedding.notStarted(missing, "JANUS_ENGINE", MISSING, "darwin", "arm64", null);
    assertTrue(message.startsWith("JANUS_ENGINE names " + missing + ", which does not exist.\n"), message);
    assertTrue(message.contains("janus-engine-aarch64-apple-darwin from"), message);
  }

  @Test
  void aPlatformWithNoReleaseIsToldToBuildOne() {
    assertTrue(SubprocessEmbedding.notStarted("janus-engine", "PATH", MISSING, "freebsd", "x64", null)
        .contains("To get one:\n  - in a pact-janus checkout, build it with"));
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
