package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.EngineError;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * A structured engine error (engine-protocol spec §10), surfaced unchanged: its {@code code},
 * {@code category}, {@code message} and {@code details} exactly as the engine sent them, and — for
 * {@code interaction-invalid} and {@code contract-invalid} — its {@code problems}, each a JSON
 * pointer into the submitted document and a message, verbatim. One type for every code: the code
 * vocabulary is open, and a code this SDK has never heard of arrives here named, with its category.
 */
public class JanusEngineException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  /** One entry of {@code details.problems}: a position in the submitted document and what is wrong there. */
  public record Problem(String pointer, String message) {
    @Override
    public String toString() {
      return pointer + ": " + message;
    }
  }

  private final String operation;
  private final String code;
  private final String category;
  private final String engineMessage;
  private final transient Map<String, Object> details;

  JanusEngineException(String operation, EngineError error) {
    super(describe(operation, error));
    this.operation = operation;
    this.code = error.getCode();
    this.category = error.getCategory();
    this.engineMessage = error.getMessage();
    this.details = error.getDetails() == null
        ? Map.of()
        : Collections.unmodifiableMap(new LinkedHashMap<>(error.getDetails()));
  }

  /** The operation that failed, e.g. {@code consumer-session/add-interaction}. */
  public String operation() {
    return operation;
  }

  /** The error code, e.g. {@code interaction-invalid}. */
  public String code() {
    return code;
  }

  /** The error category ({@code protocol}, {@code session}, {@code document}, {@code component}, {@code internal}), or null if none was sent. */
  public String category() {
    return category;
  }

  /** The engine's human-readable message. */
  public String engineMessage() {
    return engineMessage;
  }

  /** {@code details}, as the engine sent it. */
  public Map<String, Object> details() {
    return details;
  }

  /** {@code details.problems}, verbatim; empty when the error carries none. */
  public List<Problem> problems() {
    List<Problem> out = new ArrayList<>();
    if (details.get("problems") instanceof List<?> list) {
      for (Object item : list) {
        if (item instanceof Map<?, ?> m) {
          out.add(new Problem(String.valueOf(m.get("pointer")), String.valueOf(m.get("message"))));
        }
      }
    }
    return out;
  }

  /** {@code details.supported} of {@code protocol-version-unsupported}, verbatim; empty otherwise. */
  public List<Object> supported() {
    return details.get("supported") instanceof List<?> list ? List.copyOf(list) : List.of();
  }

  private static String describe(String operation, EngineError error) {
    StringBuilder text = new StringBuilder()
        .append(operation).append(" failed: ").append(error.getCode())
        .append(error.getCategory() == null ? "" : " (" + error.getCategory() + ")")
        .append(": ").append(error.getMessage());
    if (error.getDetails() != null && error.getDetails().get("problems") instanceof List<?> problems) {
      for (Object p : problems) {
        if (p instanceof Map<?, ?> m) {
          text.append("\n  ").append(m.get("pointer")).append(": ").append(m.get("message"));
        }
      }
    } else if (error.getDetails() != null && !error.getDetails().isEmpty()) {
      text.append(" ").append(error.getDetails());
    }
    return text.toString();
  }
}
