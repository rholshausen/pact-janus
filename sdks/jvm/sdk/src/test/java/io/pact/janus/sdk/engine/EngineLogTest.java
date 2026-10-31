package io.pact.janus.sdk.engine;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import ch.qos.logback.classic.Level;
import ch.qos.logback.classic.Logger;
import ch.qos.logback.classic.spi.ILoggingEvent;
import ch.qos.logback.core.read.ListAppender;
import io.pact.janus.sdk.ContractWithheldException;
import io.pact.janus.sdk.Janus;
import io.pact.janus.sdk.JanusOptions;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.slf4j.LoggerFactory;

/**
 * The engine's logs, as the JVM's own (staged plan S1.37, ADR 0061): its level is the SLF4J logger's,
 * and its events arrive there at their own levels. Logback is the provider here, as it is under
 * Spring Boot; what it is told is what a project's configuration would tell it.
 */
class EngineLogTest {
  private final Logger logger = (Logger) LoggerFactory.getLogger(EngineLog.LOGGER);
  private final ListAppender<ILoggingEvent> logged = new ListAppender<>();
  private Level before;

  @BeforeEach
  void capture() {
    before = logger.getLevel();
    logged.start();
    logger.addAppender(logged);
  }

  @AfterEach
  void release() {
    logger.detachAppender(logged);
    logger.setLevel(before);
  }

  private List<String> messages() {
    return logged.list.stream().map(event -> event.getLevel() + " " + event.getFormattedMessage()).toList();
  }

  @Test
  @DisplayName("an SLF4J provider is bound, so the engine's logs are forwarded")
  void forwarding() {
    assertTrue(EngineLog.forwarding());
  }

  @Test
  @DisplayName("the engine logs at the most detailed level the logger shows")
  void levelIsTheLoggers() {
    for (Map.Entry<Level, String> level : Map.of(
        Level.TRACE, "trace", Level.DEBUG, "debug", Level.INFO, "info", Level.WARN, "warn",
        Level.ERROR, "error", Level.OFF, "off").entrySet()) {
      logger.setLevel(level.getKey());
      assertEquals(level.getValue(), EngineLog.level(logger), level.getKey().toString());
    }
  }

  @Test
  @DisplayName("forwarded, the engine writes JSON at the logger's level, unless RUST_LOG is given")
  void environment() {
    logger.setLevel(Level.DEBUG);
    Map<String, String> derived = new HashMap<>();
    EngineLog.configure(derived, true, logger);
    assertEquals(Map.of("JANUS_LOG_FORMAT", "json", "RUST_LOG", "debug"), derived);

    Map<String, String> given = new HashMap<>(Map.of("RUST_LOG", "warn,pact_janus_component_http=trace"));
    EngineLog.configure(given, true, logger);
    assertEquals("warn,pact_janus_component_http=trace", given.get("RUST_LOG"));

    Map<String, String> unbound = new HashMap<>();
    EngineLog.configure(unbound, false, logger);
    assertEquals(Map.of("RUST_LOG", "warn"), unbound);
  }

  @Test
  @DisplayName("an event is logged at its level as target, message and fields; anything else as an error")
  void events() {
    logger.setLevel(Level.DEBUG);
    EngineLog.log("{\"timestamp\":\"2026-10-31T03:56:25Z\",\"level\":\"DEBUG\",\"fields\":{\"message\":\"the provider "
        + "answered\",\"method\":\"GET\",\"status\":200},\"target\":\"pact_janus_component_http::drive\"}", logger);
    EngineLog.log("{\"level\":\"DEBUG\",\"fields\":{\"message\":\"starting new connection\",\"log.target\":\"reqwest::connect\","
        + "\"log.module_path\":\"reqwest::connect\",\"log.line\":5},\"target\":\"reqwest::connect\"}", logger);
    EngineLog.log("{\"level\":\"TRACE\",\"fields\":{\"message\":\"held back\"},\"target\":\"x\"}", logger);
    EngineLog.log("{\"level\":\"WARN\",\"fields\":{\"message\":\"no target\"}}", logger);
    EngineLog.log("thread 'main' panicked at cli/src/bin/janus_engine.rs:1:1", logger);
    EngineLog.log("", logger);
    assertEquals(List.of(
        "DEBUG pact_janus_component_http::drive: the provider answered method=GET status=200",
        "DEBUG reqwest::connect: starting new connection",
        "WARN no target",
        "ERROR thread 'main' panicked at cli/src/bin/janus_engine.rs:1:1"), messages());
  }

  @Test
  @DisplayName("a real engine's warning reaches the logger, through the test's own SLF4J provider")
  void aRealEnginesWarning(@TempDir Path contracts) {
    logger.setLevel(Level.WARN);
    Janus janus = Janus.of("web-app", "orders-api", JanusOptions.defaults().withContractDirectory(contracts));
    janus.execute(janus.interaction("ping").request(r -> r.method("GET").path("/ping")).response(r -> r.status(204)),
        (mock, variant) -> HttpClient.newHttpClient().send(
            HttpRequest.newBuilder(URI.create(mock.url() + "/pong")).GET().build(), HttpResponse.BodyHandlers.discarding()));
    assertThrows(ContractWithheldException.class, janus::finalise);
    assertTrue(messages().stream().anyMatch(message ->
        message.startsWith("WARN pact_janus_kernel::protocol::exchange: request did not match the armed variant")),
        messages().toString());
  }
}
