// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

/** Turns single-quoted JSON into double-quoted JSON. */
public final class QuoteUtil {
  private QuoteUtil() {}

  public static String convert(String text) {
    StringBuilder builder = new StringBuilder();
    boolean singleContext = false;
    for (int i = 0; i < text.length(); i++) {
      char ch = text.charAt(i);
      if (ch == '\\') {
        i = i + 1;
        if (i < text.length()) {
          ch = text.charAt(i);
          if (!(singleContext && ch == '\'')) {
            builder.append('\\');
          }
        }
      } else if (ch == '\'') {
        ch = '"';
        singleContext = !singleContext;
      }
      builder.append(ch);
    }
    return builder.toString();
  }
}
