// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../BeforeEachTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit.MockServerConfig;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import org.apache.commons.collections4.MapUtils;
import org.apache.commons.io.IOUtils;
import org.apache.hc.client5.http.fluent.Request;
import org.apache.hc.core5.http.ClassicHttpResponse;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.equalTo;
import static org.hamcrest.Matchers.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "ArticlesProvider", pactVersion = PactSpecVersion.V3)
public class BeforeEachTest {
  private String response;
  final private String EXPECTED_RESPONSE = "expected";

  private Map<String, String> headers = MapUtils.putAll(new HashMap<>(), new String[] {
    "Content-Type", "text/plain"
  });
  
  @BeforeEach
  void beforeEach() {
      response = EXPECTED_RESPONSE;
  }

  @Pact(consumer = "Consumer")
  public RequestResponsePact pactExecutedAfterBeforeEach(PactDslWithProvider builder) {
    return builder
      .given("provider state")
      .uponReceiving("request")
        .path("/")
        .method("GET")
      .willRespondWith()
        .headers(headers)
        .status(200)
        .body(response)
      .toPact();
  }

  @Test
  @PactTestFor(pactMethod = "pactExecutedAfterBeforeEach")
  @MockServerConfig(port = "1234")
  void testPactExecutedAfterBeforeEach(MockServer mockServer) throws IOException {
    ClassicHttpResponse httpResponse = (ClassicHttpResponse) Request.get(mockServer.getUrl() + "/").execute().returnResponse();
    assertThat(IOUtils.toString(httpResponse.getEntity().getContent()),
      is(equalTo(EXPECTED_RESPONSE)));
  }
}
