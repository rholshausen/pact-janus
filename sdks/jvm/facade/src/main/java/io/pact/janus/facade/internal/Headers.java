package io.pact.janus.facade.internal;

import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

/** How pact-jvm splits a header's value and a query string. Not API. */
public final class Headers {
  private static final Set<String> MULTI_VALUE_HEADERS = Set.of("accept", "accept-encoding", "accept-language",
      "access-control-allow-headers", "access-control-allow-methods", "access-control-expose-headers",
      "access-control-request-headers", "allow", "cache-control", "if-match", "if-none-match", "vary");

  private Headers() {}

  /** A header's values: split at commas outside quotes for a header that lists values, else the one value. */
  public static List<String> values(String name, String value) {
    if (!MULTI_VALUE_HEADERS.contains(name.toLowerCase(Locale.ROOT))) {
      return List.of(value);
    }
    List<String> values = new ArrayList<>();
    StringBuilder current = new StringBuilder();
    boolean quoted = false;
    for (char c : value.toCharArray()) {
      if (c == '"') {
        quoted = !quoted;
      }
      if (c == ',' && !quoted) {
        values.add(current.toString().trim());
        current.setLength(0);
      } else {
        current.append(c);
      }
    }
    values.add(current.toString().trim());
    values.removeIf(String::isEmpty);
    return values;
  }

  /** pact-jvm's {@code queryStringToMap}: each parameter's values, in order. */
  public static Map<String, List<String>> queryStringToMap(String query, boolean decode) {
    Map<String, List<String>> map = new LinkedHashMap<>();
    if (query == null || query.isEmpty()) {
      return map;
    }
    for (String pair : query.split("&")) {
      if (pair.isEmpty()) {
        continue;
      }
      String[] nv = pair.split("=", 2);
      String name = decode ? URLDecoder.decode(nv[0], StandardCharsets.UTF_8) : nv[0];
      String value = nv.length > 1 ? (decode ? URLDecoder.decode(nv[1], StandardCharsets.UTF_8) : nv[1]) : null;
      map.computeIfAbsent(name, k -> new ArrayList<>()).add(value);
    }
    return map;
  }
}
