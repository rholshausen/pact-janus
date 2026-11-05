package io.pact.janus.facade.internal;

import io.pact.janus.facade.consumer.dsl.InvalidMatcherException;
import java.text.ParseException;
import java.text.SimpleDateFormat;
import java.time.Instant;
import java.time.ZoneOffset;
import java.time.format.DateTimeFormatter;
import java.util.Date;

/** The fixed date pact-jvm writes an example from when it is given none, and its check of one. Not API. */
public final class Dates {
  /** 2000-02-01T00:00:00 in Sydney's summer, pact-jvm's {@code DATE_2000}. */
  public static final long DATE_2000 = 949323600000L;

  private Dates() {}

  /** {@code DATE_2000} in {@code pattern}, in UTC, as pact-jvm's {@code Matchers} writes it. */
  public static String utc2000(String pattern) {
    return DateTimeFormatter.ofPattern(pattern).format(Instant.ofEpochMilli(DATE_2000).atOffset(ZoneOffset.UTC));
  }

  /** {@code DATE_2000} in {@code pattern}, in this JVM's zone, as pact-jvm's body DSL writes it. */
  public static String local2000(String pattern) {
    return new SimpleDateFormat(pattern).format(new Date(DATE_2000));
  }

  /** {@code DATE_2000} in {@code pattern} as a {@code java.time} pattern, in this JVM's zone. */
  public static String localTime2000(String pattern) {
    return DateTimeFormatter.ofPattern(pattern).withZone(java.time.ZoneId.systemDefault())
        .format(Instant.ofEpochMilli(DATE_2000));
  }

  /** {@code date} in {@code pattern}, in {@code zone}. */
  public static String format(String pattern, java.util.TimeZone zone, Date date) {
    SimpleDateFormat format = new SimpleDateFormat(pattern);
    format.setTimeZone(zone);
    return format.format(date);
  }

  /** Fails when {@code value} is not a date in {@code pattern}. */
  public static void validate(String value, String pattern) {
    if (value == null || value.isEmpty()) {
      return;
    }
    SimpleDateFormat format = new SimpleDateFormat(pattern);
    format.setLenient(false);
    try {
      format.parse(value);
    } catch (ParseException e) {
      throw new InvalidMatcherException("Example \"" + value + "\" does not match pattern \"" + pattern + "\"");
    }
  }
}
