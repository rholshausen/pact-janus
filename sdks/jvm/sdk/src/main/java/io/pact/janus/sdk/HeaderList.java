package io.pact.janus.sdk;

import java.util.List;

/**
 * A header whose value is a list, compared as its elements however its field lines frame them
 * (behavioural spec {@code header-list}, ADR 0082). Not a shape — the list is a fact about a header's
 * field lines — so only a request or response header accepts one. Made by {@link Shapes#headerList}.
 */
public final class HeaderList {
  private final List<Object> elements;

  HeaderList(List<Object> elements) {
    this.elements = List.copyOf(elements);
  }

  /** The elements, as written. */
  public List<Object> elements() {
    return elements;
  }

  static IllegalArgumentException misplaced(String where) {
    return new IllegalArgumentException(where + ": headerList() describes an HTTP header's field lines,"
        + " so it belongs directly as a request or response header's value; nothing else has field lines");
  }
}
