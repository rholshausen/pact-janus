package io.pact.janus.sdk.frameworks;

import io.pact.janus.sdk.JanusProviderExtension;
import io.pact.janus.sdk.State;
import io.pact.janus.sdk.Verifier;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.TestFactory;
import org.junit.jupiter.api.condition.EnabledIfSystemProperty;
import org.junit.jupiter.api.extension.RegisterExtension;

/** A provider test under JUnit 5: each interaction a test of its own, its states set up by methods. */
@EnabledIfSystemProperty(named = "janus.harness", matches = "true")
public class JUnitProviderTest {
  static UserService service;

  @RegisterExtension
  static final JanusProviderExtension janus = JanusProviderExtension.of(
      () -> Verifier.forProvider(service.url()).withPacts(List.of(UserService.PACT)));

  @BeforeAll
  static void start() {
    service = UserService.start();
  }

  @AfterAll
  static void stop() {
    service.close();
  }

  @State({"users exist", "a user with ID 1 exists"})
  Map<String, Object> aliceExists() {
    return service.aliceExists();
  }

  @TestFactory
  Stream<DynamicTest> pacts() {
    return janus.interactions(this);
  }
}
