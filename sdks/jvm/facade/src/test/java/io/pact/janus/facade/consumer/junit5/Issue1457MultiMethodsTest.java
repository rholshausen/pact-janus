// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../Issue1457MultiMethodsTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import org.apache.hc.client5.http.fluent.Request;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.equalTo;
import static org.hamcrest.Matchers.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "Issue1457", pactVersion = PactSpecVersion.V3)
public class Issue1457MultiMethodsTest {
  @Pact(consumer = "Issue1457Consumer")
  public RequestResponsePact countryDetails(PactDslWithProvider builder) {
    return builder
      .uponReceiving("A request to get USA code")
      .path("/code/USA")
      .willRespondWith()
      .status(200)
      .body("United States", "text/plain")
      .toPact();
  }

  @Pact(consumer = "Issue1457Consumer")
  public RequestResponsePact countryDetails2(PactDslWithProvider builder) {
    return builder
      .uponReceiving("A request to get other code")
      .path("/code/other")
      .willRespondWith()
      .status(200)
      .body("Other", "text/plain")
      .toPact();
  }

  @Test
  @PactTestFor(pactMethods = {"countryDetails", "countryDetails2"})
  @DisplayName("validate country details")
  public void getCountryDetails(MockServer mockServer) throws Exception {
    String response = Request.get(mockServer.getUrl() + "/code/USA")
      .execute().returnContent().asString();
    assertThat(response, is(equalTo("United States")));

    response = Request.get(mockServer.getUrl() + "/code/other")
      .execute().returnContent().asString();
    assertThat(response, is(equalTo("Other")));
  }
}
