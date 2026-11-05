package io.pact.janus.facade.core.support.json;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.io.UncheckedIOException;

/** A stand-in for pact-jvm's internal JSON parser, which one of pact-jvm's own tests uses. Test-only. */
public final class JsonParser {
  private JsonParser() {}

  /** The document, with pact-jvm's {@code get(name).asString()}. */
  public static Value parseString(String json) {
    try {
      return new Value(new ObjectMapper().readTree(json));
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  /** A JSON value. */
  public record Value(JsonNode node) {
    public Value get(String name) {
      return new Value(node.get(name));
    }

    public String asString() {
      return node.asText();
    }
  }
}
