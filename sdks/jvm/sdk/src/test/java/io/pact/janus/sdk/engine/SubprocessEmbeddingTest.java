package io.pact.janus.sdk.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeFalse;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

/**
 * When the SDK cannot start {@code janus-engine}, the error says why, which release asset fits this
 * platform and how to point the SDK at it: someone moving from pact-jvm has no checkout to build one in.
 */
class SubprocessEmbeddingTest {
  private static final IOException MISSING = new IOException("Cannot run program \"janus-engine\": error=2");

  @Test
  void notOnThePathSaysSoAndNamesThisPlatformsReleaseAsset() {
    assertEquals(
        "The Janus engine could not be started: there is no janus-engine on the PATH, and JANUS_ENGINE is not set.\n"
            + "The SDK runs the engine as its own executable, janus-engine, and does not ship it. Download "
            + "janus-engine-x86_64-unknown-linux-musl from " + SubprocessEmbedding.RELEASES + ", make it executable, "
            + "and put it on the PATH as janus-engine or set JANUS_ENGINE to its path. In a pact-janus checkout, "
            + "`cargo build -p pact_janus_cli --bin janus-engine` builds target/debug/janus-engine.",
        SubprocessEmbedding.notStarted("janus-engine", "PATH", MISSING, "linux", "x64"));
  }

  @Test
  void aPathThatDoesNotExistIsNamedWithWhatPointedAtIt(@TempDir Path dir) {
    String missing = dir.resolve("janus-engine").toString();
    String message = SubprocessEmbedding.notStarted(missing, "JANUS_ENGINE", MISSING, "darwin", "arm64");
    assertTrue(message.startsWith("The Janus engine could not be started: JANUS_ENGINE names " + missing
        + ", which does not exist.\n"), message);
    assertTrue(message.contains("janus-engine-aarch64-apple-darwin from"), message);
  }

  @Test
  void aPlatformWithNoReleaseIsToldToBuildOne() {
    assertTrue(SubprocessEmbedding.notStarted("janus-engine", "PATH", MISSING, "freebsd", "x64")
        .contains("No janus-engine is released for freebsd/x64. In a pact-janus checkout"));
  }

  @Test
  void startingAMissingOrNonExecutableEngineFailsWithThatError(@TempDir Path dir) throws IOException {
    Path engine = dir.resolve("janus-engine");
    IOException missing = assertThrows(IOException.class, () -> SubprocessEmbedding.of(engine).open());
    assertTrue(missing.getMessage().startsWith("The Janus engine could not be started: SubprocessEmbedding.of names "
        + engine + ", which does not exist."), missing.getMessage());
    assumeFalse(SubprocessEmbedding.platform().equals("win32"), "Windows has no executable bit");
    Files.writeString(engine, "not an executable");
    IOException notExecutable = assertThrows(IOException.class, () -> SubprocessEmbedding.of(engine).open());
    assertTrue(notExecutable.getMessage().contains(engine + " is not executable (chmod +x it)."),
        notExecutable.getMessage());
  }
}
