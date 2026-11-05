// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.matchingrules.MatchingRules;
import io.pact.janus.facade.internal.Calls;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** An HTTP interaction: a request and the response to it, as pact-jvm's DSL built them. */
public final class RequestResponseInteraction implements Interaction {
  /** One side of the exchange. */
  public record Part(
      String method,
      String path,
      Map<String, List<String>> query,
      int status,
      Map<String, List<String>> headers,
      OptionalBody body,
      MatchingRules matchingRules,
      Generators generators) {}

  private final String description;
  private final List<ProviderState> providerStates;
  private final Part request;
  private final Part response;

  public RequestResponseInteraction(String description, List<ProviderState> providerStates, Part request, Part response) {
    this.description = description;
    this.providerStates = List.copyOf(providerStates);
    this.request = request;
    this.response = response;
  }

  @Override
  public String getDescription() {
    return description;
  }

  @Override
  public List<ProviderState> getProviderStates() {
    return providerStates;
  }

  public Part getRequest() {
    return request;
  }

  public Part getResponse() {
    return response;
  }

  @Override
  public List<Map<String, Object>> calls(PactSpecVersion spec) {
    Calls calls = new Calls().call("upon_receiving", "description", description);
    calls.states(providerStates, "given", "given_with_params");
    calls.call("with_request", "method", request.method(), "path", request.path());
    request.query().forEach((name, values) -> {
      for (int index = 0; index < values.size(); index++) {
        calls.call("with_query_parameter", "name", name, "index", index, "value", values.get(index));
      }
    });
    calls.headers("request", request.headers())
        .body("request", request.body(), request.headers())
        .rulesAndGenerators("request", request.matchingRules(), request.generators(), spec);
    calls.call("response_status_v2", "status", Integer.toString(response.status()));
    calls.headers("response", response.headers())
        .body("response", response.body(), response.headers())
        .rulesAndGenerators("response", response.matchingRules(), response.generators(), spec);
    return calls.list();
  }

  @Override
  public String toString() {
    return "Interaction: " + description;
  }

  static Map<String, List<String>> copy(Map<String, List<String>> map) {
    return new LinkedHashMap<>(map);
  }
}
