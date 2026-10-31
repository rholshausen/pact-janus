package io.pact.janus.sdk.engine;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.NoSuchFileException;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;
import java.util.Map;
import java.util.Objects;

/**
 * The engine this SDK runs when it is not told which (staged plan S0.10 and S1.36, X.1; ADR 0029
 * decision 3, ADR 0060 decision 6) — the JVM spelling of the TypeScript SDK's
 * {@code src/engine/release.ts}, and the same cache, so the two share one download.
 *
 * <p>The pin is {@link #PIN}: {@code sdks/engine-release.json}, which the build copies into the jar
 * beside this class. It names a version, and the SHA-256 of the engine for each target triple.
 * Resolving is a download and a hash — the file for this platform, from that version's release, kept
 * in a cache — and a file whose digest is not the pinned one is never run and never kept.
 */
public final class EngineRelease {

  /**
   * A pin: {@code owner/name} of the repository whose releases hold the engine, the pinned
   * release (null while there is none), and release asset name → the SHA-256 it must have, in hex.
   */
  public record Pin(String repository, String version, Map<String, String> sha256) {
    public Pin {
      sha256 = sha256 == null ? Map.of() : Map.copyOf(sha256);
    }

    /** Reads a pin file. */
    public static Pin read(Path file) throws IOException {
      try (InputStream in = Files.newInputStream(file)) {
        return read(in);
      }
    }

    static Pin read(InputStream in) throws IOException {
      return new ObjectMapper()
          .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false)
          .readValue(in, Pin.class);
    }

    String pinnedVersion() {
      if (version == null || version.isEmpty()) {
        throw new IllegalStateException("No janus-engine release is pinned yet");
      }
      return version;
    }
  }

  /**
   * The pin this SDK was built with: no version, and so nothing to fetch, when the build had none
   * to copy in.
   */
  public static final Pin PIN = builtWith();

  private static Pin builtWith() {
    try (InputStream in = EngineRelease.class.getResourceAsStream("engine-release.json")) {
      return in == null ? new Pin("", null, Map.of()) : Pin.read(in);
    } catch (IOException e) {
      throw new UncheckedIOException("The engine pin in this SDK's jar does not read", e);
    }
  }

  private EngineRelease() {}

  static String platform() {
    return SubprocessEmbedding.platform();
  }

  static String arch() {
    return SubprocessEmbedding.arch();
  }

  /** What a release calls the engine for this platform (ADR 0028 decision 9). */
  public static String engineAsset() {
    return engineAsset(platform(), arch());
  }

  /** What a release calls the engine for a platform (ADR 0028 decision 9). */
  public static String engineAsset(String platform, String arch) {
    String asset = SubprocessEmbedding.releasedEngine(platform, arch);
    if (asset == null) {
      throw new IllegalStateException("No janus-engine is released for " + platform + "/" + arch);
    }
    return asset;
  }

  /** {@code JANUS_ENGINE_CACHE}, or {@code pact-janus/engines} in the user's cache directory. */
  public static Path engineCache(Map<String, String> env) {
    String configured = env.get("JANUS_ENGINE_CACHE");
    if (configured != null && !configured.isEmpty()) {
      return Path.of(configured);
    }
    Path home = Path.of(System.getProperty("user.home"));
    Path caches = switch (platform()) {
      case "win32" -> env.containsKey("LOCALAPPDATA")
          ? Path.of(env.get("LOCALAPPDATA"))
          : home.resolve("AppData").resolve("Local");
      case "darwin" -> home.resolve("Library").resolve("Caches");
      default -> env.containsKey("XDG_CACHE_HOME") ? Path.of(env.get("XDG_CACHE_HOME")) : home.resolve(".cache");
    };
    return caches.resolve("pact-janus").resolve("engines");
  }

  /**
   * {@code JANUS_ENGINE_MIRROR}, or the pinned repository's releases: a base URL under which
   * {@code v<version>/<asset>} is the engine, as GitHub serves a release's files.
   */
  public static String engineMirror(Pin pin, Map<String, String> env) {
    String configured = env.get("JANUS_ENGINE_MIRROR");
    String base = configured != null && !configured.isEmpty()
        ? configured
        : "https://github.com/" + pin.repository() + "/releases/download";
    return base.replaceAll("/+$", "");
  }

  /** Where the pinned engine for this platform is, once resolved. Reads nothing and fetches nothing. */
  public static Path enginePath(Pin pin, Path cacheDir) {
    return cacheDir.resolve(pin.pinnedVersion()).resolve(engineAsset());
  }

  private static String sha256(byte[] bytes) {
    try {
      return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException("This JVM has no SHA-256", e);
    }
  }

  /**
   * The pinned engine for this platform, as an executable on disk: from the cache if it is there
   * with the pinned digest, from the release otherwise. Throws — and leaves nothing behind — if
   * what the release serves is not what was pinned.
   *
   * @param baseUrl what a release's tag and asset name are appended to; null for
   *     {@link #engineMirror}'s
   */
  public static Path resolve(Pin pin, Path cacheDir, String baseUrl) throws IOException, InterruptedException {
    String version = pin.pinnedVersion();
    String asset = engineAsset();
    String expected = pin.sha256().get(asset);
    if (expected == null) {
      throw new IllegalStateException("janus-engine " + version + " is pinned with no digest for " + asset);
    }
    Path path = enginePath(pin, cacheDir);
    try {
      if (sha256(Files.readAllBytes(path)).equals(expected)) {
        return path;
      }
    } catch (NoSuchFileException absent) {
      // Not resolved yet.
    }

    String base = baseUrl != null ? baseUrl : engineMirror(pin, System.getenv());
    String url = base + "/v" + version + "/" + asset;
    // Once per machine and version, and long enough to look like a hang if it said nothing.
    System.err.println("pact-janus: fetching janus-engine " + version + " from " + url);
    // A release asset is served by a redirect to wherever GitHub keeps it.
    HttpClient client = HttpClient.newBuilder().followRedirects(HttpClient.Redirect.NORMAL).build();
    HttpResponse<byte[]> response;
    try {
      response = client.send(HttpRequest.newBuilder(URI.create(url)).GET().build(),
          HttpResponse.BodyHandlers.ofByteArray());
    } catch (IOException e) {
      throw new IOException("janus-engine " + version + " could not be fetched from " + url + ": "
          + Objects.requireNonNullElse(e.getMessage(), e.getClass().getSimpleName()), e);
    }
    if (response.statusCode() != 200) {
      throw new IOException("janus-engine " + version + " could not be fetched from " + url
          + ": HTTP " + response.statusCode());
    }
    byte[] bytes = response.body();
    String actual = sha256(bytes);
    if (!actual.equals(expected)) {
      throw new IOException(url + " is not the janus-engine that was pinned: its SHA-256 is " + actual
          + ", and the pin says " + expected);
    }
    // Written beside its final name and moved into place, so a run that is interrupted, or two
    // that overlap, never leave half an engine where the next one will look.
    Files.createDirectories(path.getParent());
    Path partial = path.resolveSibling(path.getFileName() + "." + ProcessHandle.current().pid() + ".partial");
    Files.write(partial, bytes);
    if (!partial.toFile().setExecutable(true, false)) {
      Files.deleteIfExists(partial);
      throw new IOException("Could not make " + partial + " executable");
    }
    Files.move(partial, path, StandardCopyOption.REPLACE_EXISTING);
    return path;
  }
}
