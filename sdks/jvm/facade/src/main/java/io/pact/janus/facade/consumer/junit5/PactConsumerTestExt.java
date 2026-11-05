// Ported from pact-jvm at 1928f2247 (4.7.8), consumer/junit5/.../PactConsumerTestExt.kt; ADR 0070.
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.ConsumerPactBuilder;
import io.pact.janus.facade.consumer.MessagePactBuilder;
import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit.MockServerConfig;
import io.pact.janus.facade.consumer.model.MockServerImplementation;
import io.pact.janus.facade.core.model.BasePact;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.V4Pact;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.pact.janus.facade.core.model.annotations.PactDirectory;
import io.pact.janus.facade.core.model.messaging.Message;
import io.pact.janus.facade.core.model.messaging.MessagePact;
import io.pact.janus.facade.internal.ProviderSession;
import io.pact.janus.facade.internal.Refused;
import io.pact.janus.sdk.Engine;
import io.pact.janus.sdk.JanusOptions;
import java.lang.reflect.Method;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.AfterTestExecutionCallback;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.BeforeTestExecutionCallback;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.ParameterResolver;
import org.junit.platform.commons.support.AnnotationSupport;
import org.junit.platform.commons.support.HierarchyTraversalMode;

/**
 * pact-jvm's JUnit 5 consumer extension over the engine (ADR 0070 decision 5). A test's {@code @Pact}
 * methods build each provider's pact; the engine serves it, through a mock for an HTTP provider, and
 * the test is handed what pact-jvm hands it. After the test each variant's verdict is reported, and
 * the contract is merged into {@code <consumer>-<provider>.janus.json} where pact-jvm writes its pacts.
 */
public class PactConsumerTestExt implements BeforeAllCallback, BeforeTestExecutionCallback,
    AfterTestExecutionCallback, AfterAllCallback, ParameterResolver, org.junit.jupiter.api.extension.TestWatcher {
  private static final ExtensionContext.Namespace NAMESPACE = ExtensionContext.Namespace.create(PactConsumerTestExt.class);

  /** The test methods JUnit disabled, whose {@code @Pact} methods need not run, as pact-jvm records them. */
  private final Set<String> disabledTestMethods = ConcurrentHashMap.newKeySet();

  @Override
  public void testDisabled(ExtensionContext context, Optional<String> reason) {
    context.getTestMethod().ifPresent(method -> disabledTestMethods.add(method.getName()));
  }

  /** One provider a test is for: its name, kind, version, mock address and {@code @Pact} methods. */
  record ProviderInfo(String name, ProviderType type, PactSpecVersion version, String host, String port,
      List<String> methods) {
    boolean http() {
      return type == ProviderType.SYNCH || type == ProviderType.UNSPECIFIED;
    }

    /** The specification the pact is written to, which is what pact-jvm writes: V4 unless the test names one. */
    PactSpecVersion written() {
      return version == PactSpecVersion.UNSPECIFIED ? PactSpecVersion.V4 : version;
    }
  }

  /** One test's engine and its providers' sessions. */
  static final class Run implements ExtensionContext.Store.CloseableResource {
    final Engine engine;
    final Map<String, ProviderSession> sessions = new LinkedHashMap<>();

    Run(Engine engine) {
      this.engine = engine;
    }

    @Override
    public void close() {
      engine.close();
    }
  }

  @Override
  public void beforeAll(ExtensionContext context) {
    context.getStore(NAMESPACE).put("executedFragments", ConcurrentHashMap.newKeySet());
  }

  @Override
  public void beforeTestExecution(ExtensionContext context) {
    if (!ignored(context)) {
      run(context);
    }
  }

  @Override
  public boolean supportsParameter(ParameterContext parameterContext, ExtensionContext context) {
    Class<?> type = parameterContext.getParameter().getType();
    List<ProviderInfo> providers = providers(context);
    if (type.isAssignableFrom(MockServer.class)) {
      return providers.stream().anyMatch(ProviderInfo::http);
    }
    if (providers.stream().anyMatch(p -> p.type() == ProviderType.ASYNCH)) {
      if (type.isAssignableFrom(List.class) || type.isAssignableFrom(MessagePact.class)
          || type.isAssignableFrom(V4Pact.class)) {
        return true;
      }
    }
    return providers.stream().anyMatch(ProviderInfo::http)
        && (type.isAssignableFrom(RequestResponsePact.class) || type.isAssignableFrom(V4Pact.class));
  }

  @Override
  public Object resolveParameter(ParameterContext parameterContext, ExtensionContext context) {
    List<ProviderInfo> providers = providers(context);
    ProviderInfo provider;
    Optional<ForProvider> forProvider = parameterContext.findAnnotation(ForProvider.class);
    if (providers.size() == 1) {
      provider = providers.get(0);
    } else if (forProvider.isPresent()) {
      provider = providers.stream().filter(p -> p.name().equals(forProvider.get().value())).findFirst()
          .orElseThrow(() -> new UnsupportedOperationException("Did not find a provider with name '"
              + forProvider.get().value() + "' for parameter: " + parameterContext.getIndex()));
    } else {
      throw new UnsupportedOperationException("You have setup multiple providers for this test. You need to specify"
          + " which provider the injected value is for with the @ForProvider annotation. Parameter: "
          + parameterContext.getIndex());
    }
    ProviderSession session = run(context).sessions.get(provider.name());
    Class<?> type = parameterContext.getParameter().getType();
    if (type.isAssignableFrom(MockServer.class)) {
      return session;
    }
    BasePact pact = session.getPact();
    if (provider.type() == ProviderType.ASYNCH) {
      List<Message> messages = session.getMessages();
      if (type.isAssignableFrom(List.class)) {
        // A V4 pact's test is handed V4 messages, as pact-jvm hands it the V4 pact's interactions.
        return pact instanceof V4Pact
            ? messages.stream().map(io.pact.janus.facade.core.model.V4Interaction.AsynchronousMessage::new).toList()
            : messages;
      }
      if (type.isAssignableFrom(MessagePact.class)) {
        return new MessagePact(pact.getProvider(), pact.getConsumer(), messages, pact.getSpecVersion());
      }
      return new V4Pact(pact.getProvider(), pact.getConsumer(), messages, PactSpecVersion.V4);
    }
    return pact;
  }

  @Override
  public void afterTestExecution(ExtensionContext context) {
    if (ignored(context)) {
      return;
    }
    Run run = context.getStore(NAMESPACE).get("run", Run.class);
    if (run == null) {
      return;
    }
    Throwable failure = context.getExecutionException().orElse(null);
    Path directory = Path.of(pactDirectory(context));
    List<String> problems = new ArrayList<>();
    for (ProviderSession session : run.sessions.values()) {
      problems.addAll(session.finish(failure, directory));
    }
    context.getStore(NAMESPACE).remove("run", Run.class).close();
    if (!problems.isEmpty()) {
      if (failure == null) {
        throw new AssertionError(String.join("\n", problems));
      }
      System.err.println(String.join("\n", problems));
    }
  }

  @Override
  public void afterAll(ExtensionContext context) {
    if (context.getExecutionException().isPresent()) {
      return;
    }
    @SuppressWarnings("unchecked")
    Set<Method> executed = (Set<Method>) context.getStore(NAMESPACE).get("executedFragments", Set.class);
    List<Method> methods = AnnotationSupport.findAnnotatedMethods(context.getRequiredTestClass(), Pact.class,
        HierarchyTraversalMode.TOP_DOWN);
    List<String> notExecuted = methods.stream()
        .filter(method -> executed == null || !executed.contains(method))
        .filter(method -> !AnnotationSupport.isAnnotated(method, Disabled.class))
        .filter(method -> !forDisabledTest(method, context))
        .map(method -> method.getDeclaringClass().getSimpleName() + "." + method.getName())
        .toList();
    if (executed != null && !notExecuted.isEmpty()) {
      throw new AssertionError("The following methods annotated with @Pact were not executed during the test: "
          + String.join(", ", notExecuted)
          + "\nIf these are currently a work in progress, add a @Disabled annotation to the method\n");
    }
  }

  // ----------------------------------------------------------------------------------------------

  /** Whether the test that names {@code pactMethod} was disabled, as pact-jvm's {@code isTestMethodsDisabled} asks. */
  private boolean forDisabledTest(Method pactMethod, ExtensionContext context) {
    for (Method method : context.getRequiredTestClass().getDeclaredMethods()) {
      PactTestFor annotation = method.getAnnotation(PactTestFor.class);
      if (annotation != null && (annotation.pactMethod().equals(pactMethod.getName())
          || List.of(annotation.pactMethods()).contains(pactMethod.getName()))) {
        return disabledTestMethods.contains(method.getName());
      }
    }
    return false;
  }

  private static boolean ignored(ExtensionContext context) {
    return context.getTestMethod().map(m -> AnnotationSupport.isAnnotated(m, PactIgnore.class)).orElse(false);
  }

  /** The test's engine and sessions, started the first time anything asks for them. */
  private Run run(ExtensionContext context) {
    ExtensionContext.Store store = context.getStore(NAMESPACE);
    Run existing = store.get("run", Run.class);
    if (existing != null) {
      return existing;
    }
    Run run = new Run(Engine.start(JanusOptions.defaults()));
    store.put("run", run);
    for (ProviderInfo provider : providers(context)) {
      BasePact pact = pact(provider, context);
      String host = provider.host() == null ? null : resolve(provider.host());
      String port = provider.port() == null ? "" : resolve(provider.port());
      run.sessions.put(provider.name(), new ProviderSession(run.engine, pact, provider.written(), provider.http(),
          host, port.isEmpty() ? 0 : Integer.parseInt(port)));
    }
    return run;
  }

  /** The providers the test is for, from {@code @PactTestFor} on the method and the class, and {@code @MockServerConfig}. */
  List<ProviderInfo> providers(ExtensionContext context) {
    ExtensionContext.Store store = context.getStore(NAMESPACE);
    @SuppressWarnings("unchecked")
    List<ProviderInfo> found = (List<ProviderInfo>) store.get("providers", List.class);
    if (found != null) {
      return found;
    }
    PactTestFor onMethod = context.getTestMethod()
        .flatMap(m -> AnnotationSupport.findAnnotation(m, PactTestFor.class)).orElse(null);
    PactTestFor onClass = classAnnotation(context, PactTestFor.class);
    for (PactTestFor annotation : new PactTestFor[] {onMethod, onClass}) {
      if (annotation != null) {
        refuse(annotation);
      }
    }
    List<String> methods = new ArrayList<>();
    PactTestFor naming = onMethod != null && (!onMethod.pactMethod().isEmpty() || onMethod.pactMethods().length > 0)
        ? onMethod : onClass;
    if (naming != null) {
      if (naming.pactMethods().length > 0) {
        methods.addAll(List.of(naming.pactMethods()));
      } else if (!naming.pactMethod().isEmpty()) {
        methods.add(naming.pactMethod());
      }
    }
    String providerName = first(onMethod, onClass, PactTestFor::providerName, "");
    ProviderType type = onMethod != null && onMethod.providerType() != ProviderType.UNSPECIFIED
        ? onMethod.providerType()
        : onClass != null ? onClass.providerType() : ProviderType.UNSPECIFIED;
    PactSpecVersion version = onMethod != null && onMethod.pactVersion() != PactSpecVersion.UNSPECIFIED
        ? onMethod.pactVersion()
        : onClass != null ? onClass.pactVersion() : PactSpecVersion.UNSPECIFIED;
    String host = first(onMethod, onClass, PactTestFor::hostInterface, "");
    String port = first(onMethod, onClass, PactTestFor::port, "");

    // Group the pact methods by the provider their @Pact names, in the order they are named.
    Map<String, List<String>> byProvider = new LinkedHashMap<>();
    if (methods.isEmpty()) {
      byProvider.put(providerName, List.of());
    } else {
      for (String method : methods) {
        String named = providerOf(method, context);
        byProvider.computeIfAbsent(named.isEmpty() ? providerName : named, k -> new ArrayList<>()).add(method);
      }
    }
    List<ProviderInfo> providers = new ArrayList<>();
    byProvider.forEach((name, pactMethods) -> {
      MockServerConfig config = mockServerConfig(context, name);
      if (config != null) {
        refuse(config);
      }
      String configuredHost = config != null && !config.hostInterface().isEmpty() ? config.hostInterface() : host;
      String configuredPort = config != null && !config.port().isEmpty() ? config.port() : port;
      providers.add(new ProviderInfo(name, type, version, configuredHost.isEmpty() ? null : configuredHost,
          configuredPort.isEmpty() ? null : configuredPort, pactMethods));
    });
    store.put("providers", providers);
    return providers;
  }

  private static final java.util.regex.Pattern EXPRESSION = java.util.regex.Pattern.compile("\\$\\{([^}]+)}");

  /** pact-jvm's expressions: each {@code ${name}} is the system property, else the environment variable, named. */
  static String resolve(String value) {
    java.util.regex.Matcher matcher = EXPRESSION.matcher(value);
    StringBuilder resolved = new StringBuilder();
    while (matcher.find()) {
      String name = matcher.group(1);
      String found = System.getProperty(name, System.getenv(name));
      matcher.appendReplacement(resolved, java.util.regex.Matcher.quoteReplacement(found == null ? "" : found));
    }
    matcher.appendTail(resolved);
    return resolved.toString();
  }

  private static String first(PactTestFor onMethod, PactTestFor onClass,
      java.util.function.Function<PactTestFor, String> value, String fallback) {
    if (onMethod != null && !value.apply(onMethod).isEmpty()) {
      return value.apply(onMethod);
    }
    if (onClass != null && !value.apply(onClass).isEmpty()) {
      return value.apply(onClass);
    }
    return fallback;
  }

  private static void refuse(PactTestFor annotation) {
    if (annotation.https() || !annotation.keyStorePath().isEmpty()) {
      throw new Refused("@PactTestFor(https = true): the pact-janus mock serves plain HTTP, so a TLS mock server is "
          + "refused (ADR 0070 decision 6)");
    }
    if (annotation.mockServerImplementation() != MockServerImplementation.Default) {
      throw new Refused("@PactTestFor(mockServerImplementation = " + annotation.mockServerImplementation()
          + "): the engine runs its own mock, so naming one is refused (ADR 0070 decision 6)");
    }
  }

  private static void refuse(MockServerConfig config) {
    if (config.tls() || !config.keyStorePath().isEmpty()) {
      throw new Refused("@MockServerConfig(tls = true): the pact-janus mock serves plain HTTP, so a TLS mock server "
          + "is refused (ADR 0070 decision 6)");
    }
    if (config.implementation() != MockServerImplementation.Default) {
      throw new Refused("@MockServerConfig(implementation = " + config.implementation()
          + "): the engine runs its own mock, so naming one is refused (ADR 0070 decision 6)");
    }
  }

  /** The {@code @MockServerConfig} for {@code provider}: the method's, else the class's, by provider when there are several. */
  private static MockServerConfig mockServerConfig(ExtensionContext context, String provider) {
    List<MockServerConfig> onMethod = context.getTestMethod()
        .map(m -> AnnotationSupport.findRepeatableAnnotations(m, MockServerConfig.class)).orElse(List.of());
    List<MockServerConfig> onClass = AnnotationSupport.findRepeatableAnnotations(context.getRequiredTestClass(),
        MockServerConfig.class);
    for (List<MockServerConfig> configs : List.of(onMethod, onClass)) {
      if (configs.size() == 1) {
        return configs.get(0);
      }
      for (MockServerConfig config : configs) {
        if (config.providerName().equals(provider)) {
          return config;
        }
      }
    }
    return null;
  }

  private static <A extends java.lang.annotation.Annotation> A classAnnotation(ExtensionContext context, Class<A> type) {
    Class<?> testClass = context.getRequiredTestClass();
    while (testClass != null) {
      Optional<A> found = AnnotationSupport.findAnnotation(testClass, type);
      if (found.isPresent()) {
        return found.get();
      }
      testClass = AnnotationSupport.isAnnotated(testClass, Nested.class) ? testClass.getEnclosingClass() : null;
    }
    return null;
  }

  private static String providerOf(String pactMethod, ExtensionContext context) {
    return AnnotationSupport.findAnnotatedMethods(context.getRequiredTestClass(), Pact.class,
            HierarchyTraversalMode.TOP_DOWN).stream()
        .filter(m -> m.getName().equals(pactMethod)).findFirst()
        .map(m -> m.getAnnotation(Pact.class).provider()).orElse("");
  }

  /** The provider's pact: each of its {@code @Pact} methods called with its builder, merged in order. */
  private BasePact pact(ProviderInfo provider, ExtensionContext context) {
    List<String> methods = provider.methods().isEmpty() ? List.of("") : provider.methods();
    BasePact merged = null;
    for (String name : methods) {
      BasePact pact = lookupPact(provider, name, context);
      merged = merged == null ? pact : merged.mergePact(pact);
    }
    return merged;
  }

  private BasePact lookupPact(ProviderInfo provider, String pactMethod, ExtensionContext context) {
    String providerName = provider.name().isEmpty() ? "default" : provider.name();
    List<Method> methods = AnnotationSupport.findAnnotatedMethods(context.getRequiredTestClass(), Pact.class,
        HierarchyTraversalMode.TOP_DOWN);
    Method method;
    if (!pactMethod.isEmpty()) {
      method = methods.stream().filter(m -> m.getName().equals(pactMethod)).findFirst().orElse(null);
    } else {
      method = methods.stream().filter(m -> {
        String named = m.getAnnotation(Pact.class).provider();
        return provider.name().isEmpty() || named.isEmpty() || named.equals(provider.name());
      }).findFirst().orElse(null);
    }
    if (method == null) {
      throw new UnsupportedOperationException("No method annotated with @Pact was found on test class "
          + context.getRequiredTestClass().getSimpleName() + " for provider '" + provider.name() + "'");
    }
    Pact annotation = method.getAnnotation(Pact.class);
    String providerToUse = annotation.provider().isEmpty() ? providerName : annotation.provider();
    Class<?> parameter = method.getParameterCount() == 1 ? method.getParameterTypes()[0] : null;
    Object builder;
    if (parameter != null && parameter.isAssignableFrom(PactDslWithProvider.class)) {
      ConsumerPactBuilder consumer = ConsumerPactBuilder.consumer(annotation.consumer());
      if (provider.version() != PactSpecVersion.UNSPECIFIED) {
        consumer.pactSpecVersion(provider.version());
      }
      builder = consumer.hasPactWith(providerToUse);
    } else if (parameter != null && parameter.isAssignableFrom(MessagePactBuilder.class)) {
      builder = new MessagePactBuilder(provider.version() == PactSpecVersion.UNSPECIFIED ? PactSpecVersion.V3
          : provider.version()).consumer(annotation.consumer()).hasPactWith(providerToUse);
    } else {
      throw new Refused("Method " + method.getName() + " takes " + (parameter == null ? "no builder" : parameter.getName())
          + ": the facade builds pacts with PactDslWithProvider and MessagePactBuilder, and pact-jvm's V4 "
          + "PactBuilder is staged plan S2.5d");
    }
    BasePact pact;
    try {
      method.setAccessible(true);
      pact = (BasePact) method.invoke(context.getRequiredTestInstance(), builder);
    } catch (ReflectiveOperationException e) {
      Throwable cause = e.getCause() == null ? e : e.getCause();
      if (cause instanceof RuntimeException runtime) {
        throw runtime;
      }
      throw new IllegalStateException(cause);
    }
    Set<Method> classExecuted = executedFragments(context);
    if (classExecuted != null) {
      classExecuted.add(method);
    }
    return pact;
  }

  @SuppressWarnings("unchecked")
  private static Set<Method> executedFragments(ExtensionContext context) {
    Optional<ExtensionContext> current = Optional.of(context);
    while (current.isPresent()) {
      Set<Method> set = (Set<Method>) current.get().getStore(NAMESPACE).get("executedFragments", Set.class);
      if (set != null) {
        return set;
      }
      current = current.get().getParent();
    }
    return null;
  }

  /** Where pact-jvm writes its pacts: {@code @PactDirectory}, else {@code pact.rootDir}, else the build tool's. */
  private static String pactDirectory(ExtensionContext context) {
    PactDirectory directory = classAnnotation(context, PactDirectory.class);
    if (directory != null) {
      return directory.value();
    }
    String fallback = System.getProperty("org.gradle.test.worker") != null ? "build/pacts" : "target/pacts";
    return System.getProperty("pact.rootDir", fallback);
  }
}
