package io.pact.janus.spike;

import com.fasterxml.jackson.databind.ObjectMapper;
import io.pact.janus.slice.EngineError;
import java.util.List;

public final class Main {
  private static final List<String> KNOWN_CODES =
      List.of("invalid-spec", "unsupported-spec-version", "session-not-found", "internal");

  public static void main(String[] args) throws Exception {
    var mapper = new ObjectMapper();
    var frame = "{\"code\":\"component-unavailable\",\"message\":\"x\",\"retryable\":true}";

    EngineError typed = mapper.readValue(frame, EngineError.class);
    System.out.printf("E1: code = '%s' verbatim; known: %s%n",
        typed.getCode(), KNOWN_CODES.contains(typed.getCode()));

    String roundTripped = mapper.writeValueAsString(typed);
    System.out.printf("E2 pass-through: round-trip = %s%n", roundTripped);
    System.out.printf("  -> unknown members %s (additionalProperties map + @JsonAnySetter, by default)%n",
        roundTripped.contains("retryable") ? "PRESERVED" : "DROPPED");
    System.out.println("RESULT: OK");
  }
}
