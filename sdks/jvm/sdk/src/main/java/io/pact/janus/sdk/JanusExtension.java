package io.pact.janus.sdk;

import java.lang.reflect.Modifier;
import java.util.Objects;
import java.util.stream.Stream;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;

/**
 * The JUnit Jupiter integration. Register one per test class, in a static field:
 *
 * <pre>{@code
 * @RegisterExtension
 * static final JanusExtension janus = JanusExtension.of("web-app", "orders-api");
 * }</pre>
 *
 * <p>It runs {@link Janus#finalise()} after the class's last test — including when a test failed —
 * so the suite's session is always released, and a withheld contract fails the class.
 *
 * <p>Tests use it either like the RFC's {@code janus.execute(interaction, closure)}, which runs every
 * variant inside one test and fails it once naming each failed variant, or as a
 * {@code @TestFactory} returning {@link #variants}, which reports every selected variant as a test
 * of its own, named by its label and id.
 */
public final class JanusExtension implements AfterAllCallback {
  private final Janus janus;

  private JanusExtension(Janus janus) {
    this.janus = Objects.requireNonNull(janus, "janus");
  }

  /** An extension around {@code Janus.of(consumer, provider)}. */
  public static JanusExtension of(String consumer, String provider) {
    return new JanusExtension(Janus.of(consumer, provider));
  }

  /** An extension around {@code Janus.of(consumer, provider, options)}. */
  public static JanusExtension of(String consumer, String provider, JanusOptions options) {
    return new JanusExtension(Janus.of(consumer, provider, options));
  }

  /** An extension around an already-configured {@link Janus}. */
  public static JanusExtension of(Janus janus) {
    return new JanusExtension(janus);
  }

  /** The configured object this extension finalises. */
  public Janus janus() {
    return janus;
  }

  /** {@link Janus#interaction}. */
  public Interaction interaction(String description) {
    return janus.interaction(description);
  }

  /** {@link Janus#execute}. */
  public void execute(Interaction interaction, VariantTest test) {
    janus.execute(interaction, test);
  }

  /**
   * {@code execute} as dynamic tests: submits the interaction now (so an invalid one fails the
   * {@code @TestFactory} before any variant runs), and returns one test per selected variant, in the
   * engine's order. Each runs {@code serve-variant} and then {@code test}, and fails with the test's
   * own exception; a failed variant still withholds the contract at {@code finalise}.
   */
  public Stream<DynamicTest> variants(Interaction interaction, VariantTest test) {
    Objects.requireNonNull(test, "test");
    Janus.Execution run = janus.begin(interaction);
    return run.variants().stream().map(variant -> DynamicTest.dynamicTest(variant.displayName(), () -> {
      VariantFailure failure = janus.run(run, variant, test);
      if (failure != null) {
        throw failure.cause();
      }
    }));
  }

  @Override
  public void afterAll(ExtensionContext context) {
    // A static extension field is inherited by @Nested classes; the session belongs to the class
    // that declared it, so only its own afterAll ends it.
    Class<?> testClass = context.getTestClass().orElse(null);
    if (testClass != null && testClass.isMemberClass() && !Modifier.isStatic(testClass.getModifiers())) {
      return;
    }
    janus.finalise();
  }
}
