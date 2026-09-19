package io.pact.janus.sdk.engine;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.EOFException;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.List;
import java.util.Locale;
import java.util.Objects;
import java.util.concurrent.TimeUnit;

/**
 * The {@code janus-engine} subprocess embedding (engine-protocol spec §3.2): frames over the
 * child's stdin/stdout with LSP-style {@code Content-Length} headers, the engine's logs on stderr
 * (inherited, so they appear with the test run's own output), and stdin EOF as the shutdown
 * signal.
 */
public final class SubprocessEmbedding implements Embedding {

  /** The environment variable the whole project uses to name the engine executable. */
  public static final String ENGINE_VARIABLE = "JANUS_ENGINE";

  private final List<String> command;

  private SubprocessEmbedding(List<String> command) {
    this.command = List.copyOf(command);
  }

  /** Runs exactly this executable. */
  public static SubprocessEmbedding of(Path executable) {
    return new SubprocessEmbedding(List.of(executable.toString()));
  }

  /**
   * Runs the executable {@value #ENGINE_VARIABLE} names, or {@code janus-engine} from the
   * {@code PATH} when it is unset.
   */
  public static SubprocessEmbedding fromEnvironment() {
    String named = System.getenv(ENGINE_VARIABLE);
    return new SubprocessEmbedding(List.of(named == null || named.isBlank() ? "janus-engine" : named));
  }

  /** The command line this embedding starts. */
  public List<String> command() {
    return command;
  }

  @Override
  public FramePipe open() throws IOException {
    ProcessBuilder builder = new ProcessBuilder(command).redirectError(ProcessBuilder.Redirect.INHERIT);
    Process process;
    try {
      process = builder.start();
    } catch (IOException e) {
      throw new IOException(
          "could not start the Janus engine " + command + " — set " + ENGINE_VARIABLE
              + " to the janus-engine executable (cargo build -p pact_janus_cli --bin janus-engine)",
          e);
    }
    return new Pipe(process);
  }

  /** The pipe to one running engine process. Serial: one request in flight at a time. */
  static final class Pipe implements FramePipe {
    private static final ObjectMapper MAPPER = new ObjectMapper();
    private final Process process;
    private final OutputStream toEngine;
    private final InputStream fromEngine;

    Pipe(Process process) {
      this.process = process;
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
    return "SubprocessEmbedding" + command;
  }

  @Override
  public boolean equals(Object o) {
    return o instanceof SubprocessEmbedding other && other.command.equals(command);
  }

  @Override
  public int hashCode() {
    return Objects.hash(command);
  }
}
