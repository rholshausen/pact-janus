package io.pact.janus.sdk;

import io.pact.janus.bindings.contract.v1.ContentTypes;
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
  /** Part -> slot -> media type, for every body written with {@code content} (contract spec §5.5). */
  private final Map<String, Map<String, String>> contentTypes = new LinkedHashMap<>();

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
   * {@code given}: appends {@code { name, params, variant-params }} to {@code states}, in call
   * order. A parameter whose value is a {@link VariantBinding} becomes a {@code variant-params}
   * binding under that name; every other parameter is passed through as written. {@code params} is
   * not written when no literal parameter remains, nor {@code variant-params} when there is no
   * binding.
   */
  public Interaction given(String name, Map<String, ?> params) {
    State state = new State();
    state.setName(Objects.requireNonNull(name, "state name"));
    Map<String, Object> literal = new LinkedHashMap<>();
    List<Map<String, Object>> bindings = new ArrayList<>();
    if (params != null) {
      for (Map.Entry<String, Object> e : Json.entries(params, "given('" + name + "') params")) {
        if (e.getValue() instanceof VariantBinding binding) {
          bindings.add(binding.binding(e.getKey()));
        } else {
          literal.put(e.getKey(), Json.value(e.getValue(), "given('" + name + "')." + e.getKey()));
        }
      }
    }
    if (!literal.isEmpty()) {
      state.setParams(literal);
    }
    if (!bindings.isEmpty()) {
      state.setVariantParams(bindings);
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
    declare("request", compiled.bodyType());
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
    declare("response", compiled.bodyType());
    return this;
  }

  private void declare(String part, String bodyType) {
    contentTypes.remove(part);
    if (bodyType != null) {
      contentTypes.put(part, Map.of("body", bodyType));
    }
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
        c.setVariantParams(s.getVariantParams() == null ? null : new ArrayList<>(s.getVariantParams()));
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
    if (!contentTypes.isEmpty()) {
      ContentTypes declared = new ContentTypes();
      contentTypes.forEach((part, slots) -> declared.setAdditionalProperty(part, new LinkedHashMap<>(slots)));
      spec.setContentTypes(declared);
    }
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
