// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../QueryWithSquareBracketsTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import org.apache.hc.client5.http.fluent.Request;
import org.apache.hc.core5.http.ClassicHttpResponse;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.io.IOException;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "QueryWithSquareBrackets", pactVersion = PactSpecVersion.V2)
public class QueryWithSquareBracketsTest {

  @Pact(consumer="test_consumer")
  public RequestResponsePact pact1(PactDslWithProvider builder) {
    return builder
      .uponReceiving("Get request with square brackets")
      .path("/")
      .method("GET")
      .matchQuery("principle_identifier[account_id]", "\\d{4}", "1234")
      .matchQuery("identifier[other]", "\\w{3}\\d{4}", "ABC1234")
      .willRespondWith()
      .status(200)
      .toPact();
  }

  @Test
  @PactTestFor(pactMethod = "pact1")
  void runTest1(MockServer mockServer) throws IOException {
    String url = mockServer.getUrl() + "?principle_identifier[account_id]=1122&identifier[other]=AAa0000";
    ClassicHttpResponse httpResponse = (ClassicHttpResponse) Request.get(url)
            .execute().returnResponse();
    assertThat(httpResponse.getCode(), is(200));
  }

  @Pact(consumer="test_consumer")
  public RequestResponsePact pact2(PactDslWithProvider builder) {
    return builder
      .uponReceiving("Put request with square brackets")
      .path("/")
      .method("PUT")
      .matchQuery("principle_identifier[account_id]", "\\d{4}", "1234")
      .matchQuery("identifier[other]", "\\w{3}\\d{4}", "ABC1234")
      .willRespondWith()
      .status(200)
      .toPact();
  }

  @Test
  @PactTestFor(pactMethod = "pact2")
  void runTest2(MockServer mockServer) throws IOException {
    String url = mockServer.getUrl() + "?principle_identifier[account_id]=1122&identifier[other]=AAa0000";
    ClassicHttpResponse httpResponse = (ClassicHttpResponse) Request.put(url)
            .execute().returnResponse();
    assertThat(httpResponse.getCode(), is(200));
  }
}
