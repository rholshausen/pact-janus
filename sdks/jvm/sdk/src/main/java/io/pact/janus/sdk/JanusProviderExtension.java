package io.pact.janus.sdk;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.NoSuchElementException;
import java.util.Objects;
import java.util.Spliterator;
import java.util.Spliterators;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.function.Supplier;
import java.util.stream.Stream;
import java.util.stream.StreamSupport;
import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;

/**
 * The JUnit Jupiter provider integration (ADR 0056 decision 3). Register one per test class, in a
 * static field, and return its interactions from a {@code @TestFactory}:
 *
 * <pre>{@code
 * @RegisterExtension
 * static final JanusProviderExtension janus = JanusProviderExtension.of(
 *     () -> Verifier.forProvider("http://localhost:" + port).withPacts(List.of(pact)));
 *
 * @State("a user with ID 1 exists")
 * void userExists(Map<String, Object> params) { ... }
 *
 * @TestFactory
 * Stream<DynamicTest> pacts() {
 *   return janus.interactions(this);
 * }
 * }</pre>
 *
 * <p>The verification is one run. Each interaction result becomes a test as the engine reports it: a
 * failure fails it with its mismatches, and a pending failure aborts it. A run that failed with no
 * failed result to show it, or that was filtered, ends with a test of its own that says so.
 */
public final class JanusProviderExtension implements AfterAllCallback {
  private final Supplier<Verifier> verifier;
  private final List<Verifier.VerificationRun> open = new CopyOnWriteArrayList<>();

  private JanusProviderExtension(Supplier<Verifier> verifier) {
    this.verifier = Objects.requireNonNull(verifier, "verifier");
  }

  /** An extension that verifies with the verifier {@code verifier} supplies, when the factory runs. */
  public static JanusProviderExtension of(Supplier<Verifier> verifier) {
    return new JanusProviderExtension(verifier);
  }

  /** An extension that verifies with {@code verifier}. */
  public static JanusProviderExtension of(Verifier verifier) {
    Objects.requireNonNull(verifier, "verifier");
    return new JanusProviderExtension(() -> verifier);
  }

  /**
   * Declares {@code test}'s {@link State} and {@link ProvidesMessage} methods on the verifier, starts
   * the run, and returns one dynamic test per interaction result, read as the run reports them.
   */
  public Stream<DynamicTest> interactions(Object test) {
    Verifier configured = Objects.requireNonNull(verifier.get(), "the supplied verifier");
    declare(configured, Objects.requireNonNull(test, "test"));
    Verifier.VerificationRun run = configured.start();
    open.add(run);
    Iterator<DynamicTest> tests = new Iterator<>() {
      private DynamicTest next;
      private boolean failedResult;
      private boolean ended;

      @Override
      public boolean hasNext() {
        if (next == null && !ended) {
          Map<String, Object> result = run.next();
          if (result != null) {
            failedResult |= "failed".equals(result.get("status")) && !Boolean.TRUE.equals(result.get("pending"));
            next = DynamicTest.dynamicTest(name(result), () -> check(result));
          } else {
            ended = true;
            next = last(run.result(), failedResult);
          }
        }
        return next != null;
      }

      @Override
      public DynamicTest next() {
        if (!hasNext()) {
          throw new NoSuchElementException();
        }
        DynamicTest test = next;
        next = null;
        return test;
      }
    };
    return StreamSupport.stream(Spliterators.spliteratorUnknownSize(tests, Spliterator.ORDERED), false)
        .onClose(() -> {
          open.remove(run);
          run.close();
        });
  }

  @Override
  public void afterAll(ExtensionContext context) {
    for (Verifier.VerificationRun run : open) {
      open.remove(run);
      run.close();
    }
  }

  /** The test's name: the consumer and the interaction's description, and the variant when not the base. */
  private static String name(Map<String, Object> result) {
    Map<?, ?> interaction = result.get("interaction") instanceof Map<?, ?> ref ? ref : Map.of();
    Map<?, ?> contract = interaction.get("contract") instanceof Map<?, ?> c ? c : Map.of();
    Object variant = result.get("variant");
    String name = contract.get("consumer") + ": " + interaction.get("description");
    return variant == null || "base".equals(variant) ? name : name + " [" + variant + "]";
  }

  private static void check(Map<String, Object> result) {
    Object status = result.get("status");
    if ("verified".equals(status)) {
      return;
    }
    String explained = VerificationFailedException.explain(result);
    if (Boolean.TRUE.equals(result.get("pending")) || "skipped".equals(status)) {
      Assumptions.abort(status + ", and pending: " + explained);
    }
    Assertions.fail(status + ": " + explained);
  }

  /** A last test, when the run's status says something no result did. */
  private static DynamicTest last(VerificationResult result, boolean failedResult) {
    if ("failed".equals(result.status()) && !failedResult) {
      return DynamicTest.dynamicTest("verification", () -> Assertions.fail("the run failed: " + result.summary()));
    }
    if ("filtered".equals(result.status())) {
      return DynamicTest.dynamicTest("verification",
          () -> Assumptions.abort("filtered: not every interaction and variant was verified"));
    }
    return null;
  }

  /** Each {@link State} and {@link ProvidesMessage} method of {@code test}'s class, as handlers on it. */
  private static void declare(Verifier verifier, Object test) {
    Map<String, Method[]> states = new LinkedHashMap<>();
    for (Method method : methods(test.getClass())) {
      State state = method.getAnnotation(State.class);
      if (state != null) {
        for (String name : state.value()) {
          states.computeIfAbsent(name, n -> new Method[2])[state.action() == State.Action.SETUP ? 0 : 1] = method;
        }
      }
      ProvidesMessage message = method.getAnnotation(ProvidesMessage.class);
      if (message != null) {
        verifier.message(message.value(), request -> message(invoke(method, test, request)));
      }
    }
    states.forEach((name, handlers) -> verifier.state(name,
        params -> handlers[0] == null ? null : values(invoke(handlers[0], test, params), handlers[0]),
        handlers[1] == null ? null : params -> invoke(handlers[1], test, params)));
  }

  /** The class's methods, then its superclasses' and interfaces', each signature once: the most specific. */
  private static List<Method> methods(Class<?> type) {
    List<Method> found = new ArrayList<>();
    List<String> seen = new ArrayList<>();
    List<Class<?>> pending = new ArrayList<>(List.of(type));
    while (!pending.isEmpty()) {
      Class<?> next = pending.remove(0);
      if (next == null || next == Object.class) {
        continue;
      }
      for (Method method : next.getDeclaredMethods()) {
        String signature = method.getName() + List.of(method.getParameterTypes());
        if (!method.isSynthetic() && !seen.contains(signature)) {
          seen.add(signature);
          found.add(method);
        }
      }
      pending.add(next.getSuperclass());
      pending.addAll(List.of(next.getInterfaces()));
    }
    return found;
  }

  private static Object invoke(Method method, Object test, Map<String, Object> argument) throws Exception {
    if (method.getParameterCount() > 1 || (method.getParameterCount() == 1 && !method.getParameterTypes()[0].isAssignableFrom(Map.class))) {
      throw new IllegalArgumentException(method + " takes no parameter, or one Map");
    }
    method.setAccessible(true);
    try {
      return method.getParameterCount() == 0 ? method.invoke(test) : method.invoke(test, argument);
    } catch (InvocationTargetException e) {
      throw e.getCause() instanceof Exception cause ? cause : e;
    }
  }

  @SuppressWarnings("unchecked")
  private static Map<String, Object> values(Object returned, Method method) {
    if (returned == null || returned instanceof Map<?, ?>) {
      return (Map<String, Object>) returned;
    }
    throw new IllegalStateException(method + " returns a Map of provider-state values, or nothing");
  }

  private static Message message(Object returned) {
    return returned instanceof Message message ? message : Message.of(Objects.requireNonNull(returned, "a produced message"));
  }
}
