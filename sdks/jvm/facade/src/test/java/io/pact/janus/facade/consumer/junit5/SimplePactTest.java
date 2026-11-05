// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../SimplePactTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MessagePactBuilder;
import io.pact.janus.facade.consumer.dsl.LambdaDsl;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.pact.janus.facade.core.model.messaging.Message;
import io.pact.janus.facade.core.model.messaging.MessagePact;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.*;

import java.util.List;
import java.util.Objects;

@ExtendWith({PactConsumerTestExt.class, SimplePactTest.MyStringResolverExtension.class})
@PactTestFor(providerName = "some-provider", providerType = ProviderType.ASYNCH, pactVersion = PactSpecVersion.V3)
class SimplePactTest {

  @BeforeAll
  static void setup(String injectedString) {
    System.out.println(injectedString);
  }

  @Pact(consumer = "some-consumer")
  public MessagePact someMessage(MessagePactBuilder builder) {
    return builder
      .expectsToReceive("message")
      .withContent(LambdaDsl.newJsonBody(object -> object.stringType("test", "Test")).build()).toPact();
  }

  @Test
  @PactTestFor(pactMethod = "someMessage")
  void consume(List<Message> messages) {
    System.out.println(messages);
  }

  static class MyStringResolverExtension implements Extension, ParameterResolver {
    @Override
    public boolean supportsParameter(ParameterContext parameterContext, ExtensionContext extensionContext) throws ParameterResolutionException {
      return Objects.equals(parameterContext.getParameter().getType(), String.class);
    }

    @Override
    public Object resolveParameter(ParameterContext parameterContext, ExtensionContext extensionContext) throws ParameterResolutionException {
      return "injectedString";
    }
  }
}
