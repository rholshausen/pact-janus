package io.pact.janus.sdk;

import com.fasterxml.jackson.core.JsonProcessingException;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Objects;

/**
 * A message a producer returns (behavioural spec {@code message-producer}, ADR 0056 decision 1): its
 * body, and optionally a content type and metadata. The body is bytes, a string, or any other value,
 * which is written as JSON. With no content type the body is unlabelled, and the engine reads its
 * type from the metadata's {@code contentType}, else from its octets.
 */
public record Message(Object body, String contentType, Map<String, Object> metadata) {
  public Message {
    metadata = metadata == null ? Map.of() : java.util.Collections.unmodifiableMap(new LinkedHashMap<>(metadata));
  }

  /** A message with this body, no content type and no metadata. */
  public static Message of(Object body) {
    return new Message(body, null, Map.of());
  }

  /** This message, labelled with {@code contentType}. */
  public Message withContentType(String contentType) {
    return new Message(body, Objects.requireNonNull(contentType, "contentType"), metadata);
  }

  /** This message, with {@code metadata} as its metadata. */
  public Message withMetadata(Map<String, ?> metadata) {
    return new Message(body, contentType, new LinkedHashMap<>(Objects.requireNonNull(metadata, "metadata")));
  }

  /**
   * This message as {@code produce-message} hooks produce one: its metadata, with the content type as
   * its {@code contentType} under the metadata given, and its body's octets, labelled only with a
   * content type. A message producer answers with it, and a shape recorder records it.
   */
  Map<String, Object> slots() {
    Map<String, Object> body = new LinkedHashMap<>();
    body.put("content", java.util.Base64.getEncoder().encodeToString(octets()));
    body.put("encoded", "base64");
    Map<String, Object> meta = new LinkedHashMap<>();
    if (contentType != null) {
      body.put("content-type", contentType);
      meta.put("contentType", contentType);
    }
    meta.putAll(metadata);
    Map<String, Object> slots = new LinkedHashMap<>();
    slots.put("metadata", Map.of("content", Json.value(meta, "metadata")));
    slots.put("body", body);
    return slots;
  }

  /** The body's octets: bytes as given, a string in UTF-8, anything else as its JSON text. */
  byte[] octets() {
    if (body instanceof byte[] bytes) {
      return bytes;
    }
    if (body instanceof String text) {
      return text.getBytes(StandardCharsets.UTF_8);
    }
    try {
      return Json.MAPPER.writeValueAsBytes(body);
    } catch (JsonProcessingException e) {
      throw new IllegalArgumentException("the message's body cannot be written as JSON: " + e.getOriginalMessage(), e);
    }
  }
}
