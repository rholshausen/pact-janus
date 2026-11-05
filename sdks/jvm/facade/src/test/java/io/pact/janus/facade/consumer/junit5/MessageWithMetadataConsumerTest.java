// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../MessageWithMetadataConsumerTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MessagePactBuilder;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.messaging.Message;
import io.pact.janus.facade.core.model.messaging.MessagePact;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.util.Collections;
import java.util.List;
import java.util.Map;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.empty;
import static org.hamcrest.Matchers.is;
import static org.hamcrest.Matchers.not;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "MessageProvider", providerType = ProviderType.ASYNCH, pactVersion = PactSpecVersion.V3)
public class MessageWithMetadataConsumerTest {

  @Pact(consumer = "test_consumer_v3")
  public MessagePact createPact(MessagePactBuilder builder) {
    PactDslJsonBody body = new PactDslJsonBody();
    body.stringValue("testParam1", "value1");
    body.stringValue("testParam2", "value2");

    return builder.given("SomeProviderState")
      .expectsToReceive("a test message with metadata")
      .withMetadata(md -> {
        md.add("metadata1", "metadataValue1");
        md.add("metadata2", "metadataValue2");
        md.add("metadata3", 10L);
        md.matchRegex("partitionKey", "[A-Z]{3}\\d{2}", "ABC01");
      })
      .withContent(body)
      .toPact();
  }

  @Test
  void test(List<Message> messages) {
    assertThat(messages, is(not(empty())));
    Message message = messages.get(0);
    Map<String, Object> metaData = message.getMetadata();
    assertThat(metaData.entrySet(), is(not(empty())));
    assertThat(metaData.get("metadata1"), is("metadataValue1"));
    assertThat(metaData.get("metadata2"), is("metadataValue2"));
    assertThat(metaData.get("metadata3"), is(10L));
    assertThat(metaData.get("partitionKey"), is("ABC01"));

    assertThat(message.getMatchingRules().rulesForCategory("metadata").allMatchingRules(),
      is(Collections.singletonList(new RegexMatcher("[A-Z]{3}\\d{2}"))));
  }
}
