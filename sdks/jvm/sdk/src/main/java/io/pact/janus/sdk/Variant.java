package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.VariantDescriptor;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The second argument of the {@code execute} closure: one variant descriptor exactly as
 * {@code consumer-session/variants} returned it (variant semantics spec §2.2, §3.9). {@link #id()}
 * is the variant's stable name; {@link #label()} is for humans only.
 */
public final class Variant {
  /** One dimension's point in a variant's assignment. */
  public record Point(String dimension, String point) {}

  private final Map<String, Object> descriptor;

  Variant(Map<String, Object> descriptor) {
    this.descriptor = Collections.unmodifiableMap(new LinkedHashMap<>(descriptor));
  }

  static Variant of(VariantDescriptor descriptor) {
    @SuppressWarnings("unchecked")
    Map<String, Object> raw = Json.MAPPER.convertValue(descriptor, LinkedHashMap.class);
    return new Variant(raw);
  }

  /** The variant id — {@code base}, or the {@code ;}-joined dimensions that differ from their default. */
  public String id() {
    return (String) descriptor.get("id");
  }

  /** The short, human-readable name; falls back to the id when the engine sent none. */
  public String label() {
    Object label = descriptor.get("label");
    return label instanceof String s ? s : id();
  }

  /** Why the variant was selected — {@code base}, {@code boundary}, {@code pinned}, {@code covering} — or null. */
  public String origin() {
    Object origin = descriptor.get("origin");
    return origin instanceof String s ? s : null;
  }

  /** The assignment: one point per active dimension, in the order the engine sent them. */
  public List<Point> assignment() {
    Object assignment = descriptor.get("assignment");
    List<Point> points = new ArrayList<>();
    if (assignment instanceof List<?> list) {
      for (Object item : list) {
        if (item instanceof Map<?, ?> m) {
          points.add(new Point(String.valueOf(m.get("dimension")), String.valueOf(m.get("point"))));
        }
      }
    }
    return Collections.unmodifiableList(points);
  }

  /**
   * The point this variant gives {@code dimension} (a dimension id such as
   * {@code response.body.shippedAt#presence}), or null when the dimension is inactive or unknown.
   */
  public String point(String dimension) {
    for (Point p : assignment()) {
      if (p.dimension().equals(dimension)) {
        return p.point();
      }
    }
    return null;
  }

  /** The whole descriptor, including members this SDK does not know. */
  public Map<String, Object> descriptor() {
    return descriptor;
  }

  /** {@code label [id]}, or just the id when the two are the same. */
  public String displayName() {
    return label().equals(id()) ? id() : label() + " [" + id() + "]";
  }

  @Override
  public String toString() {
    return displayName();
  }
}
