// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer;

import io.pact.janus.facade.consumer.dsl.DslPart;
import io.pact.janus.facade.consumer.dsl.Matcher;
import io.pact.janus.facade.consumer.xml.PactXmlBuilder;
import io.pact.janus.facade.core.model.BasePact;
import io.pact.janus.facade.core.model.Consumer;
import io.pact.janus.facade.core.model.OptionalBody;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.Provider;
import io.pact.janus.facade.core.model.ProviderState;
import io.pact.janus.facade.core.model.V4Pact;
import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.messaging.Message;
import io.pact.janus.facade.core.model.messaging.MessagePact;
import io.pact.janus.facade.internal.ContentTypes;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Builds a pact of messages. */
public class MessagePactBuilder {
  private Consumer consumer = new Consumer("consumer");
  private Provider provider = new Provider("provider");
  // pact-jvm's messages share the one list of states, so a state given after a message reaches it too.
  private final List<ProviderState> providerStates = new ArrayList<>();
  private final List<Message> messages = new ArrayList<>();
  private final PactSpecVersion specVersion;

  public MessagePactBuilder() {
    this(PactSpecVersion.V3);
  }

  public MessagePactBuilder(PactSpecVersion specVersion) {
    this.specVersion = specVersion;
  }

  public MessagePactBuilder consumer(String consumer) {
    this.consumer = new Consumer(consumer);
    return this;
  }

  public MessagePactBuilder hasPactWith(String provider) {
    this.provider = new Provider(provider);
    return this;
  }

  public MessagePactBuilder given(String providerState) {
    providerStates.add(new ProviderState(providerState));
    return this;
  }

  public MessagePactBuilder given(String providerState, Map<String, ?> params) {
    providerStates.add(new ProviderState(providerState, params));
    return this;
  }

  public MessagePactBuilder given(ProviderState providerState) {
    providerStates.add(providerState);
    return this;
  }

  public MessagePactBuilder expectsToReceive(String description) {
    messages.add(new Message(description, providerStates));
    return this;
  }

  private Message last(String call) {
    if (messages.isEmpty()) {
      throw new IllegalStateException("expectsToReceive is required before " + call);
    }
    return messages.get(messages.size() - 1);
  }

  public MessagePactBuilder withMetadata(Map<String, ?> metadata) {
    Message message = last("withMetaData");
    Map<String, Object> values = new LinkedHashMap<>();
    metadata.forEach((key, value) -> {
      if (value instanceof Matcher matcher) {
        if (matcher.getMatcher() != null) {
          message.getMatchingRules().addCategory("metadata").addRule(key, matcher.getMatcher());
        }
        if (matcher.getGenerator() != null) {
          message.getGenerators().addGenerator(Category.METADATA, matcher.getGenerator());
        }
        values.put(key, matcher.getValue());
      } else {
        values.put(key, value);
      }
    });
    message.setMetadata(values);
    return this;
  }

  public MessagePactBuilder withMetadata(java.util.function.Consumer<io.pact.janus.facade.consumer.dsl.MetadataBuilder> consumer) {
    Message message = last("withMetaData");
    io.pact.janus.facade.consumer.dsl.MetadataBuilder metadata = new io.pact.janus.facade.consumer.dsl.MetadataBuilder();
    consumer.accept(metadata);
    message.setMetadata(metadata.getValues());
    message.getMatchingRules().addCategory(metadata.getMatchers());
    metadata.getGenerators().forEach((key, generator) -> message.getGenerators().addGenerator(Category.METADATA, key, generator));
    return this;
  }

  public MessagePactBuilder withContent(DslPart body) {
    Message message = last("withContent");
    Map<String, Object> metadata = new LinkedHashMap<>(message.getMetadata());
    String contentType = contentType(metadata, "application/json");
    DslPart parent = body.close();
    message.setContents(OptionalBody.body(parent.toString().getBytes(ContentTypes.charset(contentType)), contentType));
    message.setMetadata(metadata);
    message.getMatchingRules().addCategory(parent.getMatchers());
    message.getGenerators().addGenerators(parent.getGenerators());
    return this;
  }

  public MessagePactBuilder withContent(PactXmlBuilder xmlBuilder) {
    Message message = last("withContent");
    Map<String, Object> metadata = new LinkedHashMap<>(message.getMetadata());
    String contentType = contentType(metadata, "application/xml");
    message.setContents(OptionalBody.body(xmlBuilder.asBytes(ContentTypes.charset(contentType)), contentType));
    message.setMetadata(metadata);
    message.getMatchingRules().addCategory(xmlBuilder.getMatchingRules());
    message.getGenerators().addGenerators(xmlBuilder.getGenerators());
    return this;
  }

  public MessagePactBuilder withContent(String contents) {
    return withContent(contents, "text/plain");
  }

  public MessagePactBuilder withContent(String contents, String contentType) {
    Message message = last("withContent");
    Map<String, Object> metadata = new LinkedHashMap<>(message.getMetadata());
    metadata.put("contentType", contentType);
    message.setContents(OptionalBody.body(contents.getBytes(ContentTypes.charset(contentType)), contentType));
    message.setMetadata(metadata);
    return this;
  }

  /** The metadata's content type, moved to {@code contentType}, else {@code fallback} put there. */
  private static String contentType(Map<String, Object> metadata, String fallback) {
    Map.Entry<String, Object> entry = metadata.entrySet().stream()
        .filter(e -> e.getKey().equalsIgnoreCase("contentType") || e.getKey().equalsIgnoreCase("content-type"))
        .findFirst().orElse(null);
    if (entry == null) {
      metadata.put("contentType", fallback);
      return fallback;
    }
    Object value = entry.getValue();
    metadata.remove(entry.getKey());
    metadata.put("contentType", value);
    return String.valueOf(value);
  }

  @SuppressWarnings("unchecked")
  public <P extends BasePact> P toPact(Class<P> pactClass) {
    if (pactClass.isAssignableFrom(V4Pact.class)) {
      return (P) new V4Pact(provider, consumer, messages, PactSpecVersion.V4);
    }
    if (pactClass.isAssignableFrom(MessagePact.class)) {
      return (P) new MessagePact(provider, consumer, messages, specVersion);
    }
    throw new IllegalArgumentException(pactClass.getSimpleName() + " is not a valid Pact class");
  }

  @SuppressWarnings("unchecked")
  public <P extends BasePact> P toPact() {
    return specVersion == PactSpecVersion.V4
        ? (P) new V4Pact(provider, consumer, messages, PactSpecVersion.V4)
        : (P) new MessagePact(provider, consumer, messages, specVersion);
  }
}
