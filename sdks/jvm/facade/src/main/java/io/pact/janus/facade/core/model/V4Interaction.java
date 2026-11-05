// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model;

import io.pact.janus.facade.core.model.messaging.Message;
import io.pact.janus.facade.core.model.messaging.MessageContents;
import java.util.List;
import java.util.Map;

/** pact-jvm's V4 interaction kinds, as a V4 pact's test is handed them. */
public abstract class V4Interaction implements Interaction {
  private V4Interaction() {}

  /** A V4 asynchronous message: a {@link Message} as a V4 pact holds it, its contents in one place. */
  public static final class AsynchronousMessage extends V4Interaction {
    private final Message message;

    public AsynchronousMessage(Message message) {
      this.message = message;
    }

    @Override
    public String getDescription() {
      return message.getDescription();
    }

    @Override
    public List<ProviderState> getProviderStates() {
      return message.getProviderStates();
    }

    public MessageContents getContents() {
      return new MessageContents(message.getContents(), message.getMetadata(), message.getMatchingRules(),
          message.getGenerators());
    }

    public Map<String, Object> getMetadata() {
      return message.getMetadata();
    }

    public byte[] contentsAsBytes() {
      return message.contentsAsBytes();
    }

    public String contentsAsString() {
      return message.contentsAsString();
    }

    public boolean isAsynchronousMessage() {
      return true;
    }

    @Override
    public List<Map<String, Object>> calls(PactSpecVersion spec) {
      return message.calls(spec);
    }

    @Override
    public String toString() {
      return "AsynchronousMessage(" + message + ")";
    }
  }
}
