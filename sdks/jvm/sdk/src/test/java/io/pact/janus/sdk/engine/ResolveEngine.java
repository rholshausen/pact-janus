package io.pact.janus.sdk.engine;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * {@code ResolveEngine <pin file> <out file>}: resolves the pinned engine, as the SDK does when it
 * is told nothing (staged plan S0.10), and writes its path to the out file, for Gradle to give the
 * tests as {@code JANUS_ENGINE}. Test setup, not SDK: Gradle runs it before the tests when
 * {@code JANUS_ENGINE_RELEASE} is set.
 */
public final class ResolveEngine {
  private ResolveEngine() {}

  public static void main(String[] args) throws Exception {
    if (args.length != 2) {
      System.err.println("usage: ResolveEngine <engine-release.json> <out file>");
      System.exit(2);
    }
    EngineRelease.Pin pin = EngineRelease.Pin.read(Path.of(args[0]));
    Path engine = EngineRelease.resolve(pin, EngineRelease.engineCache(System.getenv()), null);
    Path out = Path.of(args[1]);
    Files.createDirectories(out.toAbsolutePath().getParent());
    Files.writeString(out, engine.toString(), StandardCharsets.UTF_8);
    System.out.println("janus-engine " + pin.version() + " (released): " + engine);
  }
}
