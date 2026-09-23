package io.pact.janus.sdk;

import java.util.Objects;

/**
 * A body of a named media type (behavioural spec {@code content}): the slot's shape, and the type the
 * interaction declares for it (contract spec §5.5, ADR 0020). Not a shape — it names a whole slot —
 * so only {@link RequestParts#body} and {@link ResponseParts#body} accept one. Made by
 * {@link Shapes#content}.
 */
public final class Content {
  private final String mediaType;
  private final Object document;

  Content(String mediaType, Object document) {
    this.mediaType = Objects.requireNonNull(mediaType, "media type");
    this.document = document;
  }

  /** The media type, exactly as written. */
  public String mediaType() {
    return mediaType;
  }

  /** The body's shape, compiled by the literal rules. */
  public Object document() {
    return document;
  }

  static IllegalArgumentException misplaced(String where) {
    return new IllegalArgumentException(where + ": content() declares a whole slot's media type, so it"
        + " belongs directly in a request or response body; there is no slot for it to name here");
  }
}
