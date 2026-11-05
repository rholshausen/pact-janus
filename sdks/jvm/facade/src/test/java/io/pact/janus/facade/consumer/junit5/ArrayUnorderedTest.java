// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../ArrayUnorderedTest.java, its imports
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

import static io.pact.janus.facade.consumer.dsl.LambdaDsl.newJsonArrayUnordered;
import static org.junit.jupiter.api.Assertions.assertEquals;

@ExtendWith(PactConsumerTestExt.class)
public class ArrayUnorderedTest {
  @Test
  @PactTestFor(
    providerName = "PactProvider",
    pactMethod = "pactPassIdArray",
    pactVersion = PactSpecVersion.V3)
  void testContract(MockServer mockServer) throws IOException {
    HttpResponse response = Request.post(mockServer.getUrl() + "/passIdArray")
      .bodyString("[{\"id\":\"123\"}]", ContentType.APPLICATION_JSON).execute().returnResponse();
    assertEquals(response.getCode(), 200);
  }

  @Pact(consumer = "PactConsumer")
  RequestResponsePact pactPassIdArray(PactDslWithProvider provider) {
    return provider
      .uponReceiving("publish entity")
      .path("/passIdArray")
      .method("POST")
      .body(
        newJsonArrayUnordered(refs ->
          refs.object(ref -> ref.stringType("id", "123"))
        ).build()
      )
      .willRespondWith()
      .status(200)
      .toPact();
  }
}
