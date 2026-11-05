// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;

/** A body: missing, empty, or octets with the content type they were given, if any. */
public final class OptionalBody {
  /** Whether there is a body at all. */
  public enum State {
    MISSING,
    EMPTY,
    PRESENT
  }

  private final State state;
  private final byte[] value;
  private final String contentType;

  private OptionalBody(State state, byte[] value, String contentType) {
    this.state = state;
    this.value = value;
    this.contentType = contentType;
  }

  public static OptionalBody missing() {
    return new OptionalBody(State.MISSING, null, null);
  }

  public static OptionalBody empty() {
    return new OptionalBody(State.EMPTY, new byte[0], null);
  }

  public static OptionalBody body(byte[] value) {
    return body(value, null);
  }

  /** The octets, typed by {@code contentType}; none is empty, as pact-jvm's is. */
  public static OptionalBody body(byte[] value, String contentType) {
    if (value == null) {
      return missing();
    }
    if (value.length == 0) {
      return new OptionalBody(State.EMPTY, value, contentType);
    }
    return new OptionalBody(State.PRESENT, value.clone(), contentType);
  }

  public State getState() {
    return state;
  }

  public boolean isMissing() {
    return state == State.MISSING;
  }

  public boolean isEmpty() {
    return state == State.EMPTY;
  }

  public boolean isPresent() {
    return state == State.PRESENT;
  }

  public byte[] getValue() {
    return value == null ? null : value.clone();
  }

  public byte[] orEmpty() {
    return value == null ? new byte[0] : value.clone();
  }

  public String getContentType() {
    return contentType;
  }

  /** The octets as text, in the content type's charset, else UTF-8. */
  public String valueAsString() {
    if (value == null) {
      return "";
    }
    return new String(value, charset());
  }

  private Charset charset() {
    if (contentType != null) {
      for (String parameter : contentType.split(";")) {
        String[] pair = parameter.trim().split("=", 2);
        if (pair.length == 2 && pair[0].trim().equalsIgnoreCase("charset")) {
          try {
            return Charset.forName(pair[1].trim().replace("\"", ""));
          } catch (RuntimeException e) {
            return StandardCharsets.UTF_8;
          }
        }
      }
    }
    return StandardCharsets.UTF_8;
  }

  @Override
  public String toString() {
    return isPresent() ? "PRESENT(" + valueAsString() + ")" : state.name();
  }
}
