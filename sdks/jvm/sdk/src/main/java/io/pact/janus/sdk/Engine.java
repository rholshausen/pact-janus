package io.pact.janus.sdk;

import io.pact.janus.sdk.engine.FramePipe;
import java.io.IOException;
import java.util.Map;

/**
 * An engine this SDK started and greeted, for a layer that speaks the protocol itself: pact-jvm's
 * compatibility facade, which submits the calls its builders record (ADR 0070). It carries frames and
 * decides nothing; {@link Janus} is the SDK's own API.
 */
public final class Engine implements AutoCloseable {
  private final EngineClient client;

  private Engine(EngineClient client) {
    this.client = client;
  }

  /** Starts the engine {@code options} embed and sends {@code engine/hello}. */
  public static Engine start(JanusOptions options) {
    FramePipe pipe;
    try {
      pipe = options.embedding().open();
    } catch (IOException e) {
      throw new JanusEmbeddingException(e.getMessage(), e);
    }
    EngineClient client = new EngineClient(pipe);
    try {
      client.hello(Janus.SDK_NAME, Janus.SDK_VERSION);
    } catch (RuntimeException e) {
      client.close();
      throw e;
    }
    return new Engine(client);
  }

  /** Sends {@code op} and reads its {@code ok} document as {@code resultType}. */
  public <T> T call(String op, Object body, Class<T> resultType) {
    return client.call(op, body, resultType);
  }

  /** Sends {@code op} and answers its {@code ok} document. */
  public Map<String, Object> send(String op, Object body) {
    return client.send(op, body).ok();
  }

  @Override
  public void close() {
    client.close();
  }
}
