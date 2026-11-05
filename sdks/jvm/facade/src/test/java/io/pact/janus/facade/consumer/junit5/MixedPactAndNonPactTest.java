// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../MixedPactAndNonPactTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import org.apache.hc.client5.http.fluent.Request;
import org.apache.hc.core5.http.ContentType;
import org.apache.hc.core5.http.HttpResponse;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.io.IOException;

import static io.pact.janus.facade.consumer.dsl.LambdaDsl.newJsonArrayMinLike;
import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.is;
import static org.hamcrest.Matchers.not;
import static org.junit.jupiter.api.Assertions.assertEquals;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(pactVersion = PactSpecVersion.V3)
class MixedPactAndNonPactTest {
  @Pact(consumer = "ApiConsumer")
  public RequestResponsePact defaultValues(PactDslWithProvider builder) {
    return builder
      .uponReceiving("GET request to retrieve default values")
      .path("/api/test")
      .willRespondWith()
      .status(200)
      .body(newJsonArrayMinLike(1, values -> values.object(value -> {
          value.numberType("id", 32432);
          value.stringType("name", "testId254");
          value.numberType("size", 1445211);
        }
      )).build())
      .toPact();
  }

  @Test
  @PactTestFor(pactMethod = "defaultValues")
  void testDefaultValues(MockServer mockServer) throws IOException {
    HttpResponse response = Request.get(mockServer.getUrl() + "/api/test").execute().returnResponse();
    assertEquals(response.getCode(), 200);
  }

  @Test
  @PactIgnore
  void nonPactTest() {
    assertThat(true, is(not(false))); // otherwise, the universe will end
  }
}
