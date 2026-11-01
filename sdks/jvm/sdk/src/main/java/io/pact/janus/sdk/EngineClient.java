package io.pact.janus.sdk;

import io.pact.janus.bindings.protocol.v1.Capabilities;
import io.pact.janus.bindings.protocol.v1.ContractTextCapability;
import io.pact.janus.bindings.protocol.v1.EngineError;
import io.pact.janus.bindings.protocol.v1.Hello;
import io.pact.janus.bindings.protocol.v1.HelloResult;
import io.pact.janus.bindings.protocol.v1.PartyInfo;
import io.pact.janus.bindings.protocol.v1.RequestFrame;
import io.pact.janus.bindings.protocol.v1.ResponseFrame;
import io.pact.janus.bindings.protocol.v1.Vocabulary.FrameType;
import io.pact.janus.bindings.protocol.v1.Vocabulary.RequestFrameOp;
import io.pact.janus.sdk.engine.FramePipe;
import java.io.IOException;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Operations as frames: builds a request frame from a binding, sends it down the pipe, and turns
 * the response frame back into a binding or a {@link JanusEngineException}. Carries decisions
 * across the boundary and makes none. Not API.
 */
final class EngineClient implements AutoCloseable {
  /** The only protocol version this SDK speaks. */
  static final long PROTOCOL_VERSION = 1;

  /** A response frame's {@code ok} document. */
  record Response(Map<String, Object> ok) {}

  private final FramePipe pipe;
  private long nextId = 1;

  EngineClient(FramePipe pipe) {
    this.pipe = pipe;
  }

  /**
   * {@code engine/hello}, offering protocol version 1 and declaring {@code contract-text}: a
   * contract crosses the boundary as its text, so this SDK writes the engine's bytes (ADR 0063).
   */
  HelloResult hello(String hostName, String hostVersion) {
    Hello hello = new Hello();
    hello.setProtocolVersions(List.of(PROTOCOL_VERSION));
    PartyInfo host = new PartyInfo();
    host.setName(hostName);
    host.setVersion(hostVersion);
    hello.setHost(host);
    Capabilities capabilities = new Capabilities();
    capabilities.setContractText(new ContractTextCapability());
    hello.setCapabilities(capabilities);
    HelloResult result = call(RequestFrameOp.ENGINE_HELLO, hello, HelloResult.class);
    if (result.getProtocolVersion() == null || result.getProtocolVersion() != PROTOCOL_VERSION) {
      throw new JanusEmbeddingException(
          "the engine agreed protocol version " + result.getProtocolVersion() + ", but this SDK offered only "
              + PROTOCOL_VERSION);
    }
    return result;
  }

  <T> T call(String op, Object body, Class<T> resultType) {
    return Json.MAPPER.convertValue(send(op, body).ok(), resultType);
  }

  @SuppressWarnings("unchecked")
  Response send(String op, Object body) {
    String id = "r-" + nextId++;
    RequestFrame frame = new RequestFrame();
    frame.setType(FrameType.REQUEST);
    frame.setId(id);
    frame.setOp(op);
    frame.setBody(Json.MAPPER.convertValue(body, LinkedHashMap.class));
    byte[] raw;
    ResponseFrame response;
    try {
      raw = pipe.call(Json.MAPPER.writeValueAsBytes(frame));
      response = Json.MAPPER.readValue(raw, ResponseFrame.class);
    } catch (IOException e) {
      throw new JanusEmbeddingException(op + " could not reach the Janus engine: " + e.getMessage(), e);
    }
    if (response.getId() == null || response.getId().isEmpty()) {
      // An empty id is pipe-level, not an answer to this request (engine-protocol spec §4.4).
      EngineError error = response.getError();
      throw new JanusEmbeddingException("the Janus engine reported a pipe-level error while answering " + op
          + ": " + (error == null ? "(no error document)" : error.getCode() + ": " + error.getMessage()));
    }
    if (response.getError() != null) {
      throw new JanusEngineException(op, response.getError());
    }
    if (response.getOk() == null) {
      throw new JanusEmbeddingException("the Janus engine answered " + op + " with neither 'ok' nor 'error'");
    }
    return new Response(response.getOk());
  }

  @Override
  public void close() {
    pipe.close();
  }
}
