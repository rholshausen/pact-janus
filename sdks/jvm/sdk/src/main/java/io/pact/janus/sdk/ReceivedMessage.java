package io.pact.janus.sdk;

import com.fasterxml.jackson.core.JsonProcessingException;
import io.pact.janus.bindings.protocol.v1.DeliveredSlotValue;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Optional;

/**
 * The message a message test is handed (ADR 0068 decision 3): what the engine delivered for the
 * variant. Its body is octets, read as text or JSON only when the test asks; a synchronous message
 * also carries the {@link #request()} it answers.
 */
public final class ReceivedMessage {
  private final byte[] body;
  private final String contentType;
  private final Map<String, Object> metadata;
  private final ReceivedMessage request;

  @SuppressWarnings("unchecked")
  ReceivedMessage(Map<String, DeliveredSlotValue> part, ReceivedMessage request) {
    DeliveredSlotValue slot = part == null ? null : part.get("body");
    this.body = slot == null ? new byte[0] : octets(slot);
    this.contentType = slot == null ? null : slot.getContentType();
    DeliveredSlotValue meta = part == null ? null : part.get("metadata");
    this.metadata = meta != null && meta.getContent() instanceof Map<?, ?> map
        ? Collections.unmodifiableMap(new LinkedHashMap<>((Map<String, Object>) map))
        : Map.of();
    this.request = request;
  }

  /** The message {@code serve-variant} answered, with a synchronous message's request. */
  static ReceivedMessage of(Map<String, Map<String, DeliveredSlotValue>> parts) {
    Map<String, DeliveredSlotValue> sent = parts.get("request");
    return new ReceivedMessage(parts.get("message"), sent == null ? null : new ReceivedMessage(sent, null));
  }

  /** The body's octets; empty when the message has none. */
  public byte[] body() {
    return body.clone();
  }

  /** The body's media type, when it is labelled with one. */
  public Optional<String> contentType() {
    return Optional.ofNullable(contentType);
  }

  /** The message's metadata; empty when it has none. */
  public Map<String, Object> metadata() {
    return metadata;
  }

  /** A synchronous message's request: the message the consumer sends. */
  public Optional<ReceivedMessage> request() {
    return Optional.ofNullable(request);
  }

  /** The body as UTF-8 text. */
  public String text() {
    return new String(body, StandardCharsets.UTF_8);
  }

  /** The body read as JSON: maps, lists and scalars. */
  public Object json() {
    return json(Object.class);
  }

  /** The body read as JSON into {@code type}. */
  public <T> T json(Class<T> type) {
    try {
      return Json.MAPPER.readValue(body, type);
    } catch (java.io.IOException e) {
      throw new IllegalStateException("the message's body is not JSON: " + e.getMessage(), e);
    }
  }

  /** A slot's octets: base64 decoded, text as its UTF-8 encoding, any other value as its JSON text. */
  private static byte[] octets(DeliveredSlotValue slot) {
    Object content = slot.getContent();
    if ("base64".equals(slot.getEncoded()) && content instanceof String text) {
      return Base64.getDecoder().decode(text);
    }
    if ("text".equals(slot.getEncoded()) && content instanceof String text) {
      return text.getBytes(StandardCharsets.UTF_8);
    }
    try {
      return Json.MAPPER.writeValueAsBytes(content);
    } catch (JsonProcessingException e) {
      throw new IllegalStateException("the message's body cannot be written as JSON: " + e.getOriginalMessage(), e);
    }
  }

  @Override
  public String toString() {
    return "ReceivedMessage[" + (contentType == null ? "" : contentType + ", ") + body.length + " octets]";
  }
}
