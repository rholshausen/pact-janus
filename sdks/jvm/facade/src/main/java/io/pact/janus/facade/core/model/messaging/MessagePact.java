// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.messaging;

import io.pact.janus.facade.core.model.BasePact;
import io.pact.janus.facade.core.model.Consumer;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.Provider;
import java.util.ArrayList;
import java.util.List;

/** A pact of messages. */
public class MessagePact extends BasePact {
  public MessagePact(Provider provider, Consumer consumer, List<Message> messages) {
    this(provider, consumer, messages, PactSpecVersion.V3);
  }

  public MessagePact(Provider provider, Consumer consumer, List<Message> messages, PactSpecVersion version) {
    super(consumer, provider, messages, version);
  }

  /** The messages, as pact-jvm's {@code getMessages} answers them. */
  public List<Message> getMessages() {
    return getInteractions().stream().map(Message.class::cast).toList();
  }

  @Override
  public BasePact mergePact(BasePact other) {
    List<Message> merged = new ArrayList<>(getMessages());
    other.getInteractions().forEach(interaction -> merged.add((Message) interaction));
    return new MessagePact(getProvider(), getConsumer(), merged, getSpecVersion());
  }
}
