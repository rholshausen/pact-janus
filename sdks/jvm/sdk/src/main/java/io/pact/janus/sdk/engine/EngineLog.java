package io.pact.janus.sdk.engine;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.Locale;
import java.util.Map;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.slf4j.event.Level;
import org.slf4j.helpers.NOPLoggerFactory;

/**
 * The engine's logs, as the JVM's own (staged plan S1.37, ADR 0061): its events go to the SLF4J
 * logger {@value #LOGGER}, and how much it logs is what that logger is configured to show, so
 * Logback's, Log4j's or Spring Boot's own settings ({@code logging.level.io.pact.janus.engine=debug})
 * decide both. The engine writes each event as a JSON line on its stderr ({@code JANUS_LOG_FORMAT=json}),
 * and this reads its level, target and fields: nothing written for a person is parsed.
 *
 * <p>A {@code RUST_LOG} in the environment is passed as given, for directives by target; the events it
 * lets through are still logged here, at their own levels. With no SLF4J provider on the class path
 * there is nowhere to send them, so the engine's stderr is the test's, at {@code warn} unless
 * {@code RUST_LOG} says otherwise.
 */
final class EngineLog {
  /** The one logger the engine's events go to; its target is in each message. */
  static final String LOGGER = "io.pact.janus.engine";

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private EngineLog() {}

  /** Whether an SLF4J provider is bound: with none, SLF4J discards everything. */
  static boolean forwarding() {
    return !(LoggerFactory.getILoggerFactory() instanceof NOPLoggerFactory);
  }

  /**
   * Sets the engine's environment for its logs: {@code JANUS_LOG_FORMAT=json} when they are
   * forwarded, and {@code RUST_LOG} from {@code logger}'s level unless the environment has one.
   */
  static void configure(Map<String, String> environment, boolean forwarding, Logger logger) {
    if (forwarding) {
      environment.put("JANUS_LOG_FORMAT", "json");
    }
    String given = environment.get("RUST_LOG");
    if (given == null || given.isBlank()) {
      environment.put("RUST_LOG", forwarding ? level(logger) : "warn");
    }
  }

  /** The most detailed level {@code logger} shows, as {@code RUST_LOG} spells it. */
  static String level(Logger logger) {
    if (logger.isTraceEnabled()) {
      return "trace";
    } else if (logger.isDebugEnabled()) {
      return "debug";
    } else if (logger.isInfoEnabled()) {
      return "info";
    } else if (logger.isWarnEnabled()) {
      return "warn";
    } else if (logger.isErrorEnabled()) {
      return "error";
    }
    return "off";
  }

  /** Logs every line of the engine's stderr until it ends, on a daemon thread of its own. */
  static Thread forward(InputStream stderr, Logger logger) {
    Thread thread = new Thread(() -> {
      try (BufferedReader lines = new BufferedReader(new InputStreamReader(stderr, StandardCharsets.UTF_8))) {
        for (String line = lines.readLine(); line != null; line = lines.readLine()) {
          log(line, logger, System.err);
        }
      } catch (IOException ended) {
        // The engine is gone; so is anything it had left to say.
      }
    }, "janus-engine-log");
    thread.setDaemon(true);
    thread.start();
    return thread;
  }

  /**
   * Logs one line of the engine's stderr: an event at its own level, as
   * {@code target: message key=value …}. A line that is not one goes to {@code stderr} as it is,
   * where an inherited stderr would have shown it: something the engine wrote outside its logs, a
   * crash or a framing error, or the text of an engine older than {@code JANUS_LOG_FORMAT}, whose
   * level is in that text and nowhere this could read it without parsing it.
   */
  static void log(String line, Logger logger, PrintStream stderr) {
    if (line.isBlank()) {
      return;
    }
    JsonNode event = null;
    try {
      event = MAPPER.readTree(line);
    } catch (IOException notJson) {
      // Not an event: logged whole, below.
    }
    if (event == null || !event.isObject() || !event.path("level").isTextual()) {
      stderr.println(line);
      return;
    }
    Level level;
    try {
      level = Level.valueOf(event.path("level").asText().toUpperCase(Locale.ROOT));
    } catch (IllegalArgumentException unknown) {
      level = Level.INFO;
    }
    if (!logger.isEnabledForLevel(level)) {
      return;
    }
    StringBuilder message = new StringBuilder();
    String target = event.path("target").asText("");
    if (!target.isEmpty()) {
      message.append(target).append(": ");
    }
    JsonNode fields = event.path("fields");
    message.append(fields.path("message").asText(""));
    for (Map.Entry<String, JsonNode> field : fields.properties()) {
      // `log.target`, `log.module_path`, …: where a library logging through the `log` crate wrote
      // the event, which the engine's text leaves out too.
      if (!field.getKey().equals("message") && !field.getKey().startsWith("log.")) {
        JsonNode value = field.getValue();
        message.append(' ').append(field.getKey()).append('=').append(value.isTextual() ? value.asText() : value.toString());
      }
    }
    logger.atLevel(level).log(message.toString());
  }

  /** The logger the engine's events go to. */
  static Logger logger() {
    return LoggerFactory.getLogger(LOGGER);
  }
}
