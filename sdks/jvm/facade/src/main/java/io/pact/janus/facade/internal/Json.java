package io.pact.janus.facade.internal;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import java.io.IOException;
import java.io.UncheckedIOException;

/** The facade's one JSON mapper, and the path rules pact-jvm builds its rules' keys by. Not API. */
public final class Json {
  public static final ObjectMapper MAPPER = new ObjectMapper();
  public static final JsonNodeFactory NODES = JsonNodeFactory.instance;

  private static final String ALLOWED_SPECIAL_CHARS = "-_:#@";

  private Json() {}

  public static String write(Object value) {
    try {
      return MAPPER.writeValueAsString(value);
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  public static JsonNode read(String text) {
    try {
      return MAPPER.readTree(text);
    } catch (IOException e) {
      throw new UncheckedIOException(e);
    }
  }

  /** A value as the JSON tree pact-jvm's {@code Json.toJson} makes of it. */
  public static JsonNode toJson(Object value) {
    return value instanceof JsonNode node ? node : MAPPER.valueToTree(value);
  }

  /** {@code node} written as pact-jvm writes JSON: each object's members in key order. */
  public static String sortedString(JsonNode node) {
    return sorted(node).toString();
  }

  private static JsonNode sorted(JsonNode node) {
    if (node.isObject()) {
      com.fasterxml.jackson.databind.node.ObjectNode sorted = NODES.objectNode();
      java.util.List<String> names = new java.util.ArrayList<>();
      node.fieldNames().forEachRemaining(names::add);
      java.util.Collections.sort(names);
      names.forEach(name -> sorted.set(name, sorted(node.get(name))));
      return sorted;
    }
    if (node.isArray()) {
      com.fasterxml.jackson.databind.node.ArrayNode sorted = NODES.arrayNode();
      node.forEach(item -> sorted.add(sorted(item)));
      return sorted;
    }
    return node;
  }

  /** pact-jvm's {@code constructValidPath}: {@code segment} under {@code rootPath}, quoted where it must be. */
  public static String constructValidPath(String segment, String rootPath) {
    return constructValidPath(segment, rootPath, true);
  }

  public static String constructValidPath(String segment, String rootPath, boolean numbersAreIndices) {
    if (rootPath.isEmpty()) {
      return segment;
    }
    if (segment.isEmpty()) {
      return rootPath;
    }
    String root = rootPath;
    while (root.endsWith(".")) {
      root = root.substring(0, root.length() - 1);
    }
    if (numbersAreIndices && segment.chars().allMatch(Character::isDigit)) {
      return root + "[" + segment + "]";
    }
    if (!segment.equals("*") && segment.chars().anyMatch(c -> !validPathCharacter((char) c))) {
      return root + "['" + segment + "']";
    }
    return root + "." + segment;
  }

  private static boolean validPathCharacter(char c) {
    return Character.isLetterOrDigit(c) || ALLOWED_SPECIAL_CHARS.indexOf(c) >= 0;
  }
}
