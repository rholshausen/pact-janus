package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.Vocabulary.RequestFrameOp;
import io.pact.janus.sdk.engine.Embedding;
import java.io.IOException;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Base64;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;

/**
 * The recording primitives (behavioural spec {@code shape-recorder}, {@code observe},
 * {@code observe-message}, {@code finish-recording}; ADR 0086, ADR 0090). A recorder is one
 * provider-shape session: each answer the test hands it goes to the engine as the HTTP transport
 * presents one, each message as a message producer answers one, and the engine's host writes the
 * shape. Nothing here decodes a body or decides what the shape is.
 *
 * <p>Nothing is sent until the first {@link #observe}. Its source is, unless given, the class that
 * created it: a test class creates one in a {@code static} field or {@code @BeforeAll}, observes in
 * its tests and calls {@link #finish()} in {@code @AfterAll}.
 */
public final class ShapeRecorder {
  /** What the provider answered: a status, each header's field lines, and the body's octets. */
  public record Response(int status, Map<String, List<String>> headers, byte[] body, Request request) {
    public Response {
      headers = headers == null ? Map.of() : headers;
      body = body == null ? new byte[0] : body;
    }

    public Response(int status, Map<String, List<String>> headers, byte[] body) {
      this(status, headers, body, null);
    }

    /** A {@code java.net.http} response, and the request it answered. */
    public static Response of(HttpResponse<byte[]> response) {
      return new Response(response.statusCode(), response.headers().map(), response.body(),
          new Request(response.request().method(), response.request().uri().getRawPath()));
    }
  }

  /** The request an answer was to, which the engine checks the operation admits. */
  public record Request(String method, String path) {}

  /** What an observation says besides the answer. Each {@code with...} returns a copy. */
  public record Options(String description, List<String> states, Request request) {
    public static Options none() {
      return new Options(null, null, null);
    }

    public Options withDescription(String description) {
      return new Options(description, states, request);
    }

    public Options withStates(List<String> states) {
      return new Options(description, List.copyOf(states), request);
    }

    public Options withRequest(String method, String path) {
      return new Options(description, states, new Request(method, path));
    }
  }

  /**
   * What an observed message says besides the message: what a report calls its entry, which is
   * required, the provider states it was produced in, and for a synchronous message the request it
   * answered. Each {@code with...} returns a copy.
   */
  public record MessageOptions(String description, List<String> states, Message request) {
    public MessageOptions {
      if (description == null) {
        throw new IllegalArgumentException("observeMessage: name the entry with a description");
      }
    }

    /** An entry a report calls {@code description}. */
    public static MessageOptions described(String description) {
      return new MessageOptions(description, null, null);
    }

    public MessageOptions withStates(List<String> states) {
      return new MessageOptions(description, List.copyOf(states), request);
    }

    /** The message the provider was asked: the operation is then the request's. */
    public MessageOptions withRequest(Message request) {
      return new MessageOptions(description, states, Objects.requireNonNull(request, "request"));
    }
  }

  private final String provider;
  private Path shapeDir = Path.of("shapes");
  private Path evidenceDir;
  private String source;
  private Map<String, Object> policy;
  private Embedding embedding;
  private EngineClient engine;
  private String session;
  private boolean finished;

  private ShapeRecorder(String provider, String source) {
    this.provider = Objects.requireNonNull(provider, "provider");
    this.source = source;
  }

  /** A recorder for {@code provider}'s shape, whose source is the class calling this. */
  public static ShapeRecorder forProvider(String provider) {
    Class<?> caller = StackWalker.getInstance(StackWalker.Option.RETAIN_CLASS_REFERENCE).getCallerClass();
    return new ShapeRecorder(provider, caller.getName());
  }

  /** Where the shape is written, as {@code <provider>.provider-shape.json}. Default: {@code shapes}. */
  public ShapeRecorder withShapeDir(Path shapeDir) {
    this.shapeDir = Objects.requireNonNull(shapeDir, "shapeDir");
    return this;
  }

  /** Where the recorder's counts are kept. Default: {@code target/janus/evidence} beside a {@code pom.xml}, else {@code build/janus/evidence}. */
  public ShapeRecorder withEvidenceDir(Path evidenceDir) {
    this.evidenceDir = Objects.requireNonNull(evidenceDir, "evidenceDir");
    return this;
  }

  /** Whose evidence this is, replaced when it records again. */
  public ShapeRecorder withSource(String source) {
    this.source = Objects.requireNonNull(source, "source");
    return this;
  }

  /** The recorder's judgements, {@code max-options} and {@code min-evidence}, passed as given. */
  public ShapeRecorder withPolicy(Map<String, ?> policy) {
    this.policy = new LinkedHashMap<>(policy);
    return this;
  }

  /** Reaches the engine through {@code embedding} instead of the default subprocess. */
  public ShapeRecorder withEmbedding(Embedding embedding) {
    this.embedding = Objects.requireNonNull(embedding, "embedding");
    return this;
  }

  /** Records one answer to {@code operation}: a request's slots, {@code method} and {@code path} the usual two. */
  public Map<String, Object> observe(Map<String, ?> operation, Response response) {
    return observe(operation, response, Options.none());
  }

  /** Records one answer, with the engine's answer, {@code warnings} included. */
  public Map<String, Object> observe(Map<String, ?> operation, Response response, Options options) {
    String description = options.description() != null ? options.description() : describe(operation);
    Map<String, Object> slots = new LinkedHashMap<>();
    operation.forEach((slot, value) -> slots.put(slot, Literals.compile(value, "operation." + slot)));
    Map<String, Object> parts = new LinkedHashMap<>();
    parts.put("response", responseSlots(response));
    Request request = options.request() != null ? options.request() : response.request();
    if (request != null) {
      parts.put("request", Map.of("method", Map.of("content", request.method()), "path", Map.of("content", request.path())));
    }
    Map<String, Object> body = new LinkedHashMap<>();
    body.put("session", start());
    body.put("description", description);
    if (options.states() != null) {
      body.put("states", options.states());
    }
    body.put("operation", Map.of("request", slots));
    body.put("parts", parts);
    return engine.send(RequestFrameOp.PROVIDER_SHAPE_SESSION_OBSERVE, body).ok();
  }

  /**
   * Records one message the provider produced, as a message producer returns one. {@code operation}
   * names what identifies it: {@code metadata}, a map of name to template, and {@code body}, a
   * template, either or both: the message's own, or with {@link MessageOptions#withRequest} the
   * request's. Returns the engine's answer.
   */
  public Map<String, Object> observeMessage(Map<String, ?> operation, Message message, MessageOptions options) {
    if (options == null) {
      throw new IllegalArgumentException("observeMessage: name the entry with a description");
    }
    Map<String, Object> slots = new LinkedHashMap<>();
    if (operation.containsKey("metadata")) {
      slots.put("metadata", Literals.compile(operation.get("metadata"), "operation.metadata"));
    }
    if (operation.containsKey("body")) {
      slots.put("body", Literals.compile(operation.get("body"), "operation.body"));
    }
    Map<String, Object> parts = new LinkedHashMap<>();
    parts.put("message", message.slots());
    if (options.request() != null) {
      parts.put("request", options.request().slots());
    }
    Map<String, Object> body = new LinkedHashMap<>();
    body.put("session", start());
    body.put("kind", "message");
    body.put("description", options.description());
    if (options.states() != null) {
      body.put("states", options.states());
    }
    body.put("operation", Map.of(options.request() != null ? "request" : "message", slots));
    body.put("parts", parts);
    return engine.send(RequestFrameOp.PROVIDER_SHAPE_SESSION_OBSERVE, body).ok();
  }

  /** Has the engine write the shape, merged with every other source's, and ends it. Returns the path, or null when nothing was observed. */
  public Path finish() {
    if (finished || engine == null) {
      finished = true;
      return null;
    }
    finished = true;
    try {
      if (source == null) {
        throw new IllegalArgumentException("finish: name the recorder's 'source'");
      }
      Path cwd = Path.of(System.getProperty("user.dir"));
      Path evidence = evidenceDir != null ? evidenceDir
          : Path.of(Files.exists(cwd.resolve("pom.xml")) ? "target" : "build", "janus", "evidence");
      Map<String, Object> write = new LinkedHashMap<>();
      write.put("path", cwd.resolve(shapeDir).resolve(provider + ".provider-shape.json").normalize().toString());
      write.put("evidence", cwd.resolve(evidence).resolve(provider + ".json").normalize().toString());
      write.put("source", source);
      Object written = engine.send(RequestFrameOp.PROVIDER_SHAPE_SESSION_FINALISE, Map.of("session", session, "write", write))
          .ok().get("written");
      return Path.of(String.valueOf(written));
    } finally {
      engine.close();
    }
  }

  private String start() {
    if (engine == null) {
      try {
        engine = new EngineClient((embedding == null ? Embedding.defaultEmbedding() : embedding).open());
      } catch (IOException e) {
        throw new JanusEmbeddingException(e.getMessage(), e);
      }
      engine.hello(Janus.SDK_NAME, Janus.SDK_VERSION);
      Map<String, Object> create = new LinkedHashMap<>();
      create.put("provider", Map.of("name", provider));
      if (policy != null) {
        create.put("policy", policy);
      }
      session = (String) engine.send(RequestFrameOp.PROVIDER_SHAPE_SESSION_CREATE, create).ok().get("session");
    }
    return session;
  }

  /** {@code GET /orders/{id}}: the operation's plain method and plain path or path template. */
  private static String describe(Map<String, ?> operation) {
    Object method = operation.get("method");
    Object path = operation.get("path");
    String text = path instanceof String s ? s
        : path instanceof ShapeNode node && "http:path".equals(node.operator())
            ? String.valueOf(node.binding().getAdditionalProperties().get("template")) : null;
    if (!(method instanceof String) || text == null) {
      throw new IllegalArgumentException("observe: name the entry with a description, since the operation's method"
          + " and path are not a plain value and a plain value or path template");
    }
    return method + " " + text;
  }

  /** The answer as the HTTP transport presents a response (component-interfaces spec §5.9). */
  private static Map<String, Object> responseSlots(Response response) {
    Map<String, Object> slots = new LinkedHashMap<>();
    slots.put("status", Map.of("content", response.status()));
    Map<String, List<String>> lines = new LinkedHashMap<>();
    response.headers().forEach((name, values) ->
        lines.computeIfAbsent(name.toLowerCase(Locale.ROOT), n -> new ArrayList<>()).addAll(values));
    if (!lines.isEmpty()) {
      slots.put("headers", Map.of("content", lines));
    }
    if (response.body().length > 0) {
      Map<String, Object> body = new LinkedHashMap<>();
      body.put("content", Base64.getEncoder().encodeToString(response.body()));
      body.put("encoded", "base64");
      List<String> contentType = lines.get("content-type");
      if (contentType != null && !contentType.isEmpty()) {
        body.put("content-type", contentType.get(0));
      }
      slots.put("body", body);
    }
    return slots;
  }
}
