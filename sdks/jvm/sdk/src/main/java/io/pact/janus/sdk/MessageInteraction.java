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
 * A message interaction under construction ({@code message}, {@code given},
 * {@code message-contents}, {@code message-request}; ADR 0068): a message the provider emits and the
 * consumer receives. Its transport is {@code { kind: message, mode: emissive }}. No protocol call
 * happens until {@link Janus#execute(MessageInteraction, MessageTest)} submits {@link #toSpec()}.
 */
public final class MessageInteraction {
  private final String description;
  private final List<State> states = new ArrayList<>();
  /** Part -> its slots, {@code request} before {@code message}. */
  private final Map<String, Map<String, Shape>> parts = new LinkedHashMap<>();
  private final Map<String, String> bodyTypes = new LinkedHashMap<>();

  MessageInteraction(String description) {
    this.description = Objects.requireNonNull(description, "message description");
  }

  public String description() {
    return description;
  }

  /** {@code given}, as {@link Interaction#given(String)}. */
  public MessageInteraction given(String name) {
    return given(name, null);
  }

  /** {@code given}, as {@link Interaction#given(String, Map)}. */
  public MessageInteraction given(String name, Map<String, ?> params) {
    states.add(Interaction.state(name, params));
    return this;
  }

  /** {@code message-contents}: the message delivered, {@code parts.message}. Replaces an earlier one. */
  public MessageInteraction contents(Consumer<MessageSlots> slots) {
    return declare("message", slots);
  }

  /** {@code message-request}: the request a synchronous message answers, {@code parts.request}. */
  public MessageInteraction request(Consumer<MessageSlots> slots) {
    return declare("request", slots);
  }

  private MessageInteraction declare(String part, Consumer<MessageSlots> slots) {
    MessageSlots compiled = new MessageSlots(part);
    Objects.requireNonNull(slots, part + " slots").accept(compiled);
    parts.put(part, compiled.compile());
    bodyTypes.remove(part);
    if (compiled.bodyType() != null) {
      bodyTypes.put(part, compiled.bodyType());
    }
    return this;
  }

  /** The interaction-spec document, built afresh on each call. */
  @SuppressWarnings("unchecked")
  public InteractionSpec toSpec() {
    InteractionSpec spec = new InteractionSpec();
    spec.setDescription(description);
    Transport transport = new Transport();
    transport.setKind("message");
    transport.setMode("emissive");
    spec.setTransport(transport);
    if (!states.isEmpty()) {
      spec.setStates(Interaction.copy(states));
    }
    Map<String, ShapePart> built = new LinkedHashMap<>();
    for (String part : List.of("request", "message")) {
      Map<String, Shape> slots = parts.get(part);
      if (slots != null) {
        ShapePart shapePart = new ShapePart();
        slots.forEach((name, shape) ->
            shapePart.setAdditionalProperty(name, Json.MAPPER.convertValue(shape, LinkedHashMap.class)));
        built.put(part, shapePart);
      }
    }
    spec.setParts(built);
    if (!bodyTypes.isEmpty()) {
      ContentTypes declared = new ContentTypes();
      bodyTypes.forEach((part, type) -> declared.setAdditionalProperty(part, Map.of("body", type)));
      spec.setContentTypes(declared);
    }
    return spec;
  }

  /** The document {@link #toSpec()} builds, as JSON-shaped maps and lists. */
  @SuppressWarnings("unchecked")
  public Map<String, Object> toDocument() {
    return Json.MAPPER.convertValue(toSpec(), Map.class);
  }

  @Override
  public String toString() {
    return "MessageInteraction[" + description + "]";
  }
}
