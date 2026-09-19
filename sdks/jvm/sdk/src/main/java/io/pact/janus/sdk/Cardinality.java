package io.pact.janus.sdk;

import java.util.OptionalLong;

/**
 * The options bag of {@code eachLike}: {@code { min, max }}, each only when given. Absent, the shape
 * itself means {@code min} 1 and {@code max} unbounded (shape spec §4.3), so nothing is written for
 * a bound that was not given.
 */
public final class Cardinality {
  private final Long min;
  private final Long max;

  private Cardinality(Long min, Long max) {
    this.min = min;
    this.max = max;
  }

  /** Neither bound given. */
  public static Cardinality unspecified() {
    return new Cardinality(null, null);
  }

  /** {@code { min }}. */
  public static Cardinality min(long min) {
    return new Cardinality(min, null);
  }

  /** {@code { max }}. */
  public static Cardinality max(long max) {
    return new Cardinality(null, max);
  }

  /** {@code { min, max }}. */
  public static Cardinality between(long min, long max) {
    return new Cardinality(min, max);
  }

  /** This, with {@code min} given. */
  public Cardinality withMin(long min) {
    return new Cardinality(min, max);
  }

  /** This, with {@code max} given. */
  public Cardinality withMax(long max) {
    return new Cardinality(min, max);
  }

  public OptionalLong minimum() {
    return min == null ? OptionalLong.empty() : OptionalLong.of(min);
  }

  public OptionalLong maximum() {
    return max == null ? OptionalLong.empty() : OptionalLong.of(max);
  }

  Long minOrNull() {
    return min;
  }

  Long maxOrNull() {
    return max;
  }

  @Override
  public String toString() {
    return "{" + (min == null ? "" : "min: " + min) + (min != null && max != null ? ", " : "")
        + (max == null ? "" : "max: " + max) + "}";
  }
}
