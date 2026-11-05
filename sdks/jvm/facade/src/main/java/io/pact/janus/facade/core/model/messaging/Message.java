// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.messaging;

import io.pact.janus.facade.core.model.Interaction;
import io.pact.janus.facade.core.model.OptionalBody;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.ProviderState;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.matchingrules.MatchingRules;
import io.pact.janus.facade.core.model.matchingrules.MatchingRulesImpl;
import io.pact.janus.facade.internal.Calls;
import io.pact.janus.facade.internal.Json;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * A message a provider sends: its contents and metadata, with their rules and generators. A test is
 * handed the message the engine served, its generators' values filled.
 */
public class Message implements Interaction {
  private final String description;
  private final List<ProviderState> providerStates;
  private OptionalBody contents;
  private Map<String, Object> metadata;
  private final MatchingRules matchingRules;
  private final Generators generators;

  public Message(String description, List<ProviderState> providerStates) {
    this(description, providerStates, OptionalBody.missing(), new MatchingRulesImpl(), new Generators(),
        new LinkedHashMap<>());
  }

  public Message(String description, List<ProviderState> providerStates, OptionalBody contents,
      MatchingRules matchingRules, Generators generators, Map<String, Object> metadata) {
    this.description = description;
    this.providerStates = providerStates;
    this.contents = contents;
    this.matchingRules = matchingRules;
    this.generators = generators;
    this.metadata = new LinkedHashMap<>(metadata);
  }

  @Override
  public String getDescription() {
    return description;
  }

  @Override
  public List<ProviderState> getProviderStates() {
    return providerStates;
  }

  public OptionalBody getContents() {
    return contents;
  }

  public void setContents(OptionalBody contents) {
    this.contents = contents;
  }

  public Map<String, Object> getMetadata() {
    return metadata;
  }

  public void setMetadata(Map<String, Object> metadata) {
    this.metadata = new LinkedHashMap<>(metadata);
  }

  public MatchingRules getMatchingRules() {
    return matchingRules;
  }

  public Generators getGenerators() {
    return generators;
  }

  public byte[] contentsAsBytes() {
    return contents.orEmpty();
  }

  public String contentsAsString() {
    return contents.valueAsString();
  }

  /** The content type the metadata names, else the contents'. */
  public String getContentType() {
    for (Map.Entry<String, Object> entry : metadata.entrySet()) {
      if (entry.getKey().equalsIgnoreCase("contentType") || entry.getKey().equalsIgnoreCase("content-type")) {
        return String.valueOf(entry.getValue());
      }
    }
    return contents.getContentType();
  }

  /** This message as served: the contents and metadata the engine handed over. Not API. */
  public Message served(OptionalBody contents, Map<String, Object> metadata) {
    return new Message(description, new ArrayList<>(providerStates), contents, matchingRules, generators, metadata);
  }

  @Override
  public List<Map<String, Object>> calls(PactSpecVersion spec) {
    Calls calls = new Calls().call("new_message", "description", description);
    calls.states(providerStates, "message_given", "given_with_params");
    calls.body("request", contents);
    metadata.forEach((name, value) -> calls.call("message_with_metadata_v2", "name", name, "value", Json.write(value)));
    calls.rulesAndGenerators("request", matchingRules, generators, spec);
    return calls.list();
  }

  @Override
  public String toString() {
    return "Message(description=" + description + ", metadata=" + metadata + ", contents=" + contents + ")";
  }
}
