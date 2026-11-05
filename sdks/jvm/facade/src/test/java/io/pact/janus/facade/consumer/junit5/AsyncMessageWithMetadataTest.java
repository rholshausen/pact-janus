// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../AsyncMessageWithMetadataTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MessagePactBuilder;
import io.pact.janus.facade.consumer.dsl.Matchers;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.core.model.V4Interaction;
import io.pact.janus.facade.core.model.V4Pact;
import io.pact.janus.facade.core.model.annotations.Pact;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.util.List;
import java.util.Map;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.hasEntry;
import static org.hamcrest.core.Is.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "AmqpProviderWithMetadata", providerType = ProviderType.ASYNCH)
public class AsyncMessageWithMetadataTest {

  @Pact(consumer = "test_consumer")
  public V4Pact withMetadata(MessagePactBuilder builder) {
    return builder
            .given("Some State")
            .expectsToReceive("A message with metadata")
            .withMetadata(Map.of("someKey", Matchers.string("someString")))
            .withContent(new PactDslJsonBody().stringType("someField", "someValue"))
            .toPact(V4Pact.class);
  }

  @Test
  @PactTestFor
  void test(List<V4Interaction.AsynchronousMessage> messages) {
    assertThat(new String(messages.get(0).contentsAsBytes()), is("{\"someField\":\"someValue\"}"));
    assertThat(messages.get(0).getMetadata(), hasEntry("someKey", "someString"));
  }
}
