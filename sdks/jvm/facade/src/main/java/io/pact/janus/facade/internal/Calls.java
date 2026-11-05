package io.pact.janus.facade.internal;

import io.pact.janus.facade.core.model.OptionalBody;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.ProviderState;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.matchingrules.MatchingRules;
import java.util.ArrayList;
import java.util.Base64;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The {@code pact_ffi} calls that build an interaction pact-jvm built: its example, its rules and its
 * generators as pact-jvm wrote them, each sent as it stands (ADR 0070 decision 2). Not API.
 */
public final class Calls {
  private final List<Map<String, Object>> calls = new ArrayList<>();

  public List<Map<String, Object>> list() {
    return calls;
  }

  public Calls call(String name, Object... arguments) {
    Map<String, Object> call = new LinkedHashMap<>();
    call.put("call", name);
    for (int i = 0; i < arguments.length; i += 2) {
      if (arguments[i + 1] != null) {
        call.put((String) arguments[i], arguments[i + 1]);
      }
    }
    calls.add(call);
    return this;
  }

  public Calls states(List<ProviderState> states, String given, String givenWithParams) {
    for (ProviderState state : states) {
      if (state.getParams().isEmpty() || givenWithParams == null) {
        call(given, "description", state.getName());
      } else {
        call(givenWithParams, "description", state.getName(), "params", state.getParams());
      }
    }
    return this;
  }

  /** Each value of each header, at its index. */
  public Calls headers(String part, Map<String, List<String>> headers) {
    headers.forEach((name, values) -> {
      for (int index = 0; index < values.size(); index++) {
        call("with_header_v2", "part", part, "name", name, "index", index, "value", values.get(index));
      }
    });
    return this;
  }

  /** The body's octets as they stand, typed as given or, when not, as detected. */
  public Calls body(String part, OptionalBody body) {
    return body(part, body, Map.of());
  }

  /**
   * The body's octets as they stand, typed as given, else by the part's {@code Content-Type}, else as
   * detected, which is the type pact-jvm writes the body with. No {@code Content-Type} header is added:
   * the part has every header pact-jvm gave it, and pact-jvm types a body it detects without one.
   */
  public Calls body(String part, OptionalBody body, Map<String, List<String>> headers) {
    if (body.isPresent() || body.isEmpty() && body.getContentType() != null) {
      String declared = headers.entrySet().stream()
          .filter(header -> header.getKey().equalsIgnoreCase("Content-Type") && !header.getValue().isEmpty())
          .map(header -> header.getValue().get(0)).findFirst().orElse(null);
      String contentType = body.getContentType() != null ? body.getContentType()
          : declared != null ? declared : contentType(body);
      call("with_binary_body", "part", part, "content_type", contentType, "content_type_header", false,
          "body", Base64.getEncoder().encodeToString(body.orEmpty()));
    }
    return this;
  }

  /** The rules and generators, by category, as a pact of {@code spec} writes them. */
  public Calls rulesAndGenerators(String part, MatchingRules rules, Generators generators, PactSpecVersion spec) {
    if (rules != null && rules.isNotEmpty()) {
      call("with_matching_rules", "part", part, "rules", rules.toMap(spec));
    }
    if (generators != null && generators.isNotEmpty()) {
      call("with_generators", "part", part, "generators", generators.toMap(spec));
    }
    return this;
  }

  /** The type a body was given, else JSON when its text is JSON, else plain text, as pact-jvm detects one. */
  static String contentType(OptionalBody body) {
    if (body.getContentType() != null) {
      return body.getContentType();
    }
    String text = body.valueAsString().trim();
    if (text.startsWith("{") || text.startsWith("[")) {
      try {
        Json.read(text);
        return "application/json";
      } catch (RuntimeException e) {
        return "text/plain";
      }
    }
    if (text.startsWith("<")) {
      return "application/xml";
    }
    return "text/plain";
  }
}
