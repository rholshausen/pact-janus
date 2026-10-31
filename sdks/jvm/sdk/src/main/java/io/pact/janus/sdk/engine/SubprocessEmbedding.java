package io.pact.janus.sdk.engine;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.EOFException;
import java.io.IOException;
import java.io.InputStream;
import java.io.InterruptedIOException;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.InvalidPathException;
import java.nio.file.Path;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.TimeUnit;

/**
 * The {@code janus-engine} subprocess embedding (engine-protocol spec §3.2): frames over the
 * child's stdin/stdout with LSP-style {@code Content-Length} headers, the engine's logs on stderr,
 * handed to SLF4J ({@link EngineLog}), and stdin EOF as the shutdown signal.
 */
public final class SubprocessEmbedding implements Embedding {

  /** The environment variable the whole project uses to name the engine executable. */
  public static final String ENGINE_VARIABLE = "JANUS_ENGINE";

  /** Where the engine is released. */
  public static final String RELEASES = "https://github.com/" + EngineRelease.PIN.repository() + "/releases";

  private static final Map<String, String> TRIPLES = Map.of(
      "linux/x64", "x86_64-unknown-linux-musl",
      "linux/arm64", "aarch64-unknown-linux-musl",
      "darwin/x64", "x86_64-apple-darwin",
      "darwin/arm64", "aarch64-apple-darwin",
      "win32/x64", "x86_64-pc-windows-msvc");

  private static final String BUILD = "in a pact-janus checkout, build it with `cargo build -p pact_janus_cli --bin "
      + "janus-engine` and set " + ENGINE_VARIABLE + " to target/debug/janus-engine";

  /** Empty for the pinned release, which is resolved when the engine is started. */
  private final List<String> command;
  /** What pointed at the executable, as an error names it: {@code release} when nothing did. */
  private final String source;
  /** Where the pinned release is cached and fetched from: {@code JANUS_ENGINE_CACHE}, {@code JANUS_ENGINE_MIRROR}. */
  private final Map<String, String> env;

  private SubprocessEmbedding(List<String> command, String source, Map<String, String> env) {
    this.command = List.copyOf(command);
    this.source = source;
    this.env = env;
  }

  /** Runs exactly this executable. */
  public static SubprocessEmbedding of(Path executable) {
    return new SubprocessEmbedding(List.of(executable.toString()), "SubprocessEmbedding.of", Map.of());
  }

  /**
   * Runs the executable {@value #ENGINE_VARIABLE} names, or, when it is unset, the engine this SDK
   * was released with, fetched on first use ({@link EngineRelease}; ADR 0060 decision 6).
   */
  public static SubprocessEmbedding fromEnvironment() {
    return fromEnvironment(System.getenv());
  }

  static SubprocessEmbedding fromEnvironment(Map<String, String> env) {
    String named = env.get(ENGINE_VARIABLE);
    return named == null || named.isBlank()
        ? new SubprocessEmbedding(List.of(), "release", Map.copyOf(env))
        : new SubprocessEmbedding(List.of(named), ENGINE_VARIABLE, Map.of());
  }

  /** This JVM's platform, spelled as Node spells it: {@code linux}, {@code darwin}, {@code win32}. */
  static String platform() {
    String name = System.getProperty("os.name").toLowerCase(Locale.ROOT);
    if (name.startsWith("windows")) {
      return "win32";
    }
    if (name.startsWith("mac")) {
      return "darwin";
    }
    return name;
  }

  /** This JVM's architecture, spelled as Node spells it: {@code x64}, {@code arm64}. */
  static String arch() {
    String arch = System.getProperty("os.arch").toLowerCase(Locale.ROOT);
    return switch (arch) {
      case "amd64", "x86_64" -> "x64";
      case "aarch64", "arm64" -> "arm64";
      default -> arch;
    };
  }

  /**
   * What a release calls the engine for a platform (ADR 0028 decision 9), or null when none is
   * released for it.
   */
  public static String releasedEngine(String platform, String arch) {
    String triple = TRIPLES.get(platform + "/" + arch);
    return triple == null ? null : "janus-engine-" + triple + (platform.equals("win32") ? ".exe" : "");
  }

  /**
   * Why {@code executable} could not be started, then each way to get an engine, naming this
   * platform's release asset. Wrapped at spaces, never inside a URL or a `quoted` command, to
   * {@code width} columns when it is given.
   */
  static String notStarted(
      String executable, String source, IOException cause, String platform, String arch, Integer width) {
    String reason;
    Path path = asPath(executable);
    if (path != null && !Files.exists(path)) {
      reason = source.equals("release")
          ? executable + " does not exist"
          : source + " names " + executable + ", which does not exist";
    } else if (path != null && !Files.isExecutable(path)) {
      reason = executable + " is not executable" + (platform.equals("win32") ? "" : " (chmod +x it)");
    } else {
      reason = executable + " could not be run: " + cause.getMessage();
    }
    List<String> fixes = source.equals("release")
        ? List.of()
        : List.of((source.equals(ENGINE_VARIABLE) ? "unset " : "leave out ") + source
            + ", and the SDK fetches the janus-engine it was released with; or");
    return laidOut(reason, "The SDK runs the Janus engine as its own executable.", fixes, platform, arch, width);
  }

  /** The pinned engine could not be fetched ({@code reason}, from {@link EngineRelease}): why, and how to get one. */
  static String notFetched(String reason, String platform, String arch, Integer width) {
    List<String> fixes = releasedEngine(platform, arch) != null
        ? List.of("set JANUS_ENGINE_MIRROR to a server holding janus-engine's release files, as "
            + "`<mirror>/v<version>/<file>`; or")
        : List.of();
    return laidOut(reason, "The SDK fetches the Janus engine it was released with when it first runs.", fixes,
        platform, arch, width);
  }

  private static String laidOut(
      String reason, String intro, List<String> fixes, String platform, String arch, Integer width) {
    String asset = releasedEngine(platform, arch);
    List<String> all = new java.util.ArrayList<>(fixes);
    if (asset != null) {
      all.add("download " + asset + " from " + RELEASES + "/tag/v" + EngineRelease.PIN.version()
          + ", make it executable, and set " + ENGINE_VARIABLE + " to its path; or");
      all.add(BUILD + ".");
    } else {
      all.add(BUILD + ". No janus-engine is released for " + platform + "/" + arch + ".");
    }
    List<String> lines = new java.util.ArrayList<>(wrap(reason + ".", width, "", ""));
    lines.add("");
    lines.addAll(wrap(intro + " " + (all.size() > 1 ? "Either" : "To get one") + ":", width, "", ""));
    all.forEach(fix -> lines.addAll(wrap(fix, width, "  - ", "    ")));
    return String.join("\n", lines);
  }

  /** The terminal's width, less a test report's indent, when {@code COLUMNS} says it; else null. */
  static Integer terminalWidth() {
    try {
      int columns = Integer.parseInt(Objects.requireNonNullElse(System.getenv("COLUMNS"), ""));
      return Math.min(columns, 120) - 8;
    } catch (NumberFormatException e) {
      return null;
    }
  }

  private static List<String> wrap(String text, Integer width, String first, String rest) {
    List<String> lines = new java.util.ArrayList<>();
    StringBuilder line = new StringBuilder(first);
    int words = 0;
    java.util.regex.Matcher word = java.util.regex.Pattern.compile("`[^`]*`\\S*|\\S+").matcher(text);
    while (word.find()) {
      if (width != null && words > 0 && line.length() + 1 + word.group().length() > width) {
        lines.add(line.toString());
        line = new StringBuilder(rest);
        words = 0;
      }
      line.append(words > 0 ? " " : "").append(word.group());
      words++;
    }
    lines.add(line.toString());
    return lines;
  }

  private static Path asPath(String executable) {
    try {
      return Path.of(executable);
    } catch (InvalidPathException e) {
      return null;
    }
  }

  /** The command line this embedding starts: empty for the pinned release, resolved as it starts. */
  public List<String> command() {
    return command;
  }

  @Override
  public FramePipe open() throws IOException {
    List<String> started = command.isEmpty() ? List.of(resolveRelease().toString()) : command;
    boolean forwarding = EngineLog.forwarding();
    org.slf4j.Logger logger = EngineLog.logger();
    ProcessBuilder builder = new ProcessBuilder(started)
        .redirectError(forwarding ? ProcessBuilder.Redirect.PIPE : ProcessBuilder.Redirect.INHERIT);
    EngineLog.configure(builder.environment(), forwarding, logger);
    Process process;
    try {
      process = builder.start();
    } catch (IOException e) {
      throw new EngineNotStartedException(notStarted(started.get(0), source, e, platform(), arch(), terminalWidth()));
    }
    return new Pipe(process, forwarding ? EngineLog.forward(process.getErrorStream(), logger) : null);
  }

  /** The engine this SDK was released with, from the cache or fetched into it. */
  private Path resolveRelease() throws IOException {
    EngineRelease.Pin pin = EngineRelease.PIN;
    try {
      return EngineRelease.resolve(pin, EngineRelease.engineCache(env), EngineRelease.engineMirror(pin, env));
    } catch (IOException | IllegalStateException e) {
      throw new EngineNotStartedException(notFetched(e.getMessage(), platform(), arch(), terminalWidth()));
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new InterruptedIOException("interrupted fetching janus-engine " + pin.version());
    }
  }

  /** The pipe to one running engine process. Serial: one request in flight at a time. */
  static final class Pipe implements FramePipe {
    private static final ObjectMapper MAPPER = new ObjectMapper();
    private final Process process;
    private final OutputStream toEngine;
    private final InputStream fromEngine;
    /** What hands the engine's logs to SLF4J, or null when they go to the test's stderr. */
    private final Thread log;

    Pipe(Process process, Thread log) {
      this.process = process;
      this.log = log;
      this.toEngine = process.getOutputStream();
      this.fromEngine = new BufferedInputStream(process.getInputStream());
    }

    @Override
    public synchronized byte[] call(byte[] requestFrame) throws IOException {
      String id = MAPPER.readTree(requestFrame).path("id").asText();
      writeFrame(toEngine, requestFrame);
      while (true) {
        byte[] frame = readFrame(fromEngine);
        if (frame == null) {
          throw new EOFException("the Janus engine exited" + exitDescription() + " before answering request '" + id + "'");
        }
        JsonNode node;
        try {
          node = MAPPER.readTree(frame);
        } catch (IOException e) {
          throw new IOException("the Janus engine wrote a frame that is not JSON: " + preview(frame), e);
        }
        // Only a response answers a request. Event frames (push delivery is never negotiated
        // here) and frame types this SDK does not know are ignored on stdio (§4.5).
        if (!"response".equals(node.path("type").asText())) {
          continue;
        }
        String answered = node.path("id").asText();
        if (answered.equals(id) || answered.isEmpty()) {
          return frame;
        }
      }
    }

    @Override
    public synchronized void close() {
      try {
        toEngine.close(); // stdin EOF: the engine releases every session and exits (§3.2, §6)
      } catch (IOException ignored) {
        // already gone
      }
      try {
        if (!process.waitFor(5, TimeUnit.SECONDS)) {
          process.destroy();
          if (!process.waitFor(2, TimeUnit.SECONDS)) {
            process.destroyForcibly();
          }
        }
        if (log != null) {
          // What the engine said as it exited is logged before close returns.
          log.join(1000);
        }
      } catch (InterruptedException e) {
        process.destroyForcibly();
        Thread.currentThread().interrupt();
      }
    }

    private String exitDescription() {
      try {
        if (process.waitFor(1, TimeUnit.SECONDS)) {
          return " (exit code " + process.exitValue() + ")";
        }
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
      }
      return "";
    }
  }

  static void writeFrame(OutputStream out, byte[] frame) throws IOException {
    out.write(("Content-Length: " + frame.length + "\r\n\r\n").getBytes(StandardCharsets.US_ASCII));
    out.write(frame);
    out.flush();
  }

  /** Reads one framed body, or {@code null} at a clean end of stream. Unknown headers are ignored. */
  static byte[] readFrame(InputStream in) throws IOException {
    Integer length = null;
    boolean sawAnything = false;
    while (true) {
      String line = readHeaderLine(in, sawAnything);
      if (line == null) {
        return null;
      }
      sawAnything = true;
      if (line.isEmpty()) {
        break;
      }
      int colon = line.indexOf(':');
      if (colon > 0 && line.substring(0, colon).trim().toLowerCase(Locale.ROOT).equals("content-length")) {
        length = Integer.parseInt(line.substring(colon + 1).trim());
      }
    }
    if (length == null) {
      throw new IOException("the Janus engine sent a frame header with no Content-Length");
    }
    byte[] body = in.readNBytes(length);
    if (body.length != length) {
      throw new EOFException("the Janus engine's stdout ended inside a frame");
    }
    return body;
  }

  private static String readHeaderLine(InputStream in, boolean midHeader) throws IOException {
    ByteArrayOutputStream line = new ByteArrayOutputStream();
    while (true) {
      int b = in.read();
      if (b == -1) {
        if (!midHeader && line.size() == 0) {
          return null;
        }
        throw new EOFException("the Janus engine's stdout ended inside a frame header");
      }
      if (b == '\n') {
        byte[] bytes = line.toByteArray();
        int end = bytes.length > 0 && bytes[bytes.length - 1] == '\r' ? bytes.length - 1 : bytes.length;
        return new String(bytes, 0, end, StandardCharsets.US_ASCII);
      }
      line.write(b);
    }
  }

  private static String preview(byte[] frame) {
    String text = new String(frame, StandardCharsets.UTF_8);
    return text.length() > 200 ? text.substring(0, 200) + "…" : text;
  }

  @Override
  public String toString() {
    return "SubprocessEmbedding" + (command.isEmpty() ? "[janus-engine " + EngineRelease.PIN.version() + "]" : command);
  }

  @Override
  public boolean equals(Object o) {
    return o instanceof SubprocessEmbedding other && other.command.equals(command) && other.env.equals(env);
  }

  @Override
  public int hashCode() {
    return Objects.hash(command, env);
  }
}
