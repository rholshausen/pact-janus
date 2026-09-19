package io.pact.janus.sdk;

import io.pact.janus.bindings.contract.v1.InteractionSpec;
import io.pact.janus.bindings.contract.v1.ShapePart;
import io.pact.janus.bindings.contract.v1.State;
import io.pact.janus.bindings.contract.v1.Transport;
import io.pact.janus.bindings.shape.v1.Shape;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.function.Consumer;

/**
 * An interaction specification under construction ({@code interaction}, {@code given},
 * {@code request}, {@code response}). Exists only in the idiomatic layer: no protocol call happens
 * until {@link Janus#execute} submits the document {@link #toSpec()} builds.
 *
 * <p>The transport is {@code { kind: http, mode: passive }}: a request and a response describe an
 * HTTP exchange the consumer initiates.
 */
public final class Interaction {
  private final String description;
  private final List<State> states = new ArrayList<>();
  private Map<String, Shape> request;
  private Map<String, Shape> response;

  Interaction(String description) {
    this.description = Objects.requireNonNull(description, "interaction description");
  }

  public String description() {
    return description;
  }

  /** {@code given} with no parameters: appends {@code { name }} to {@code states}. */
  public Interaction given(String name) {
    return given(name, null);
  }

  /**
   * {@code given}: appends {@code { name, params }} to {@code states}, in call order. The
   * parameters are passed through as written; {@code null} means not given, and then no
   * {@code params} member is written at all.
   */
  public Interaction given(String name, Map<String, ?> params) {
    State state = new State();
    state.setName(Objects.requireNonNull(name, "state name"));
    if (params != null) {
      @SuppressWarnings("unchecked")
      Map<String, Object> json = (Map<String, Object>) Json.value(params, "given('" + name + "') params");
      state.setParams(json);
    }
    states.add(state);
    return this;
  }

  /**
   * {@code request}: declares {@code parts.request}. Replaces any earlier {@code request} on this
   * interaction.
   */
  public Interaction request(Consumer<RequestParts> parts) {
    RequestParts compiled = new RequestParts();
    Objects.requireNonNull(parts, "request parts").accept(compiled);
    this.request = compiled.compile(); // a malformed value is reported here, at the call
    return this;
  }

  /**
   * {@code response}: declares {@code parts.response}. Replaces any earlier {@code response} on
   * this interaction.
   */
  public Interaction response(Consumer<ResponseParts> parts) {
    ResponseParts compiled = new ResponseParts();
    Objects.requireNonNull(parts, "response parts").accept(compiled);
    this.response = compiled.compile();
    return this;
  }

  /**
   * The interaction-spec document (contract spec §1, {@code $defs/InteractionSpec}), built afresh on
   * each call: building the same chain twice produces two equal documents.
   */
  public InteractionSpec toSpec() {
    InteractionSpec spec = new InteractionSpec();
    spec.setDescription(description);
    Transport transport = new Transport();
    transport.setKind("http");
    transport.setMode("passive");
    spec.setTransport(transport);
    if (!states.isEmpty()) {
      List<State> copy = new ArrayList<>();
      for (State s : states) {
        State c = new State();
        c.setName(s.getName());
        c.setParams(s.getParams() == null ? null : new LinkedHashMap<>(s.getParams()));
        copy.add(c);
      }
      spec.setStates(copy);
    }
    Map<String, ShapePart> parts = new LinkedHashMap<>();
    if (request != null) {
      parts.put("request", part(request));
    }
    if (response != null) {
      parts.put("response", part(response));
    }
    spec.setParts(parts);
    return spec;
  }

  /** The document {@link #toSpec()} builds, as JSON-shaped maps and lists. */
  @SuppressWarnings("unchecked")
  public Map<String, Object> toDocument() {
    return Json.MAPPER.convertValue(toSpec(), Map.class);
  }

  @SuppressWarnings("unchecked")
  private static ShapePart part(Map<String, Shape> slots) {
    ShapePart part = new ShapePart();
    for (Map.Entry<String, Shape> e : slots.entrySet()) {
      part.setAdditionalProperty(e.getKey(), Json.MAPPER.convertValue(e.getValue(), LinkedHashMap.class));
    }
    return part;
  }

  @Override
  public String toString() {
    return "Interaction[" + description + "]";
  }
}
