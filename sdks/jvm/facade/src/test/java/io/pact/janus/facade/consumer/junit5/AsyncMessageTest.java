// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../AsyncMessageTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MessagePactBuilder;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.pact.janus.facade.consumer.dsl.Matchers;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.core.model.messaging.Message;
import io.pact.janus.facade.core.model.messaging.MessagePact;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.hasEntry;
import static org.hamcrest.core.Is.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "MessageProvider", providerType = ProviderType.ASYNCH, pactVersion = PactSpecVersion.V3)
public class AsyncMessageTest {

  @Pact(consumer = "test_consumer_v3")
  MessagePact createPact(MessagePactBuilder builder) {
    PactDslJsonBody body = new PactDslJsonBody();
    body.stringValue("testParam1", "value1");
    body.stringValue("testParam2", "value2");

    Map<String, Object> metadata = new HashMap<>();
    metadata.put("destination", Matchers.regexp("\\w+\\d+", "X001"));

    return builder.given("SomeProviderState")
      .expectsToReceive("a test message")
      .withMetadata(metadata)
      .withContent(body)
      .toPact();
  }

  @Pact(provider = "MessageProvider", consumer = "test_consumer_v3")
  MessagePact createPact2(MessagePactBuilder builder) {
    PactDslJsonBody body = new PactDslJsonBody();
    body.stringValue("testParam1", "value3");
    body.stringValue("testParam2", "value4");

    Map<String, String> metadata = new HashMap<String, String>();
    metadata.put("Content-Type", "application/json");

    return builder.given("SomeProviderState2")
      .expectsToReceive("a test message")
      .withMetadata(metadata)
      .withContent(body)
      .toPact();
  }

  @Test
  @PactTestFor(pactMethod = "createPact")
  void test(List<Message> messages) {
    assertThat(new String(messages.get(0).contentsAsBytes()), is("{\"testParam1\":\"value1\",\"testParam2\":\"value2\"}"));
    assertThat(messages.get(0).getMetadata(), hasEntry("destination", "X001"));
  }

  @Test
  @PactTestFor(pactMethod = "createPact2")
  void test2(MessagePact pact) {
    assertThat(new String(pact.getMessages().get(0).contentsAsBytes()), is("{\"testParam1\":\"value3\",\"testParam2\":\"value4\"}"));
  }
}
