// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../UrlEncocdedFormPostTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.FormPostBuilder;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import org.apache.hc.client5.http.fluent.Request;
import org.apache.hc.core5.http.HttpResponse;
import org.apache.hc.core5.http.message.BasicNameValuePair;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.DisabledIfEnvironmentVariable;
import org.junit.jupiter.api.extension.ExtendWith;

import java.io.IOException;
import java.util.UUID;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.equalTo;
import static org.hamcrest.Matchers.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "FormPostProvider", pactVersion = PactSpecVersion.V3)
// TODO: form-urlencoded bodies not supported yet
@DisabledIfEnvironmentVariable(named = "PACT_MATCHING_ENGINE", matches = "v2",
  disabledReason = "form-urlencoded bodies not supported yet")
public class UrlEncocdedFormPostTest {
  @Pact(consumer = "FormPostConsumer")
  public RequestResponsePact formpost(PactDslWithProvider builder) {
    return builder
      .uponReceiving("FORM POST request")
        .path("/form")
        .method("POST")
        .body(
          new FormPostBuilder()
            .uuid("id")
            .stringMatcher("value", "\\d+", "1", "2", "3"))
      .willRespondWith()
        .status(200)
      .toPact();
  }

  @Test
  void testFormPost(MockServer mockServer) throws IOException {
    HttpResponse httpResponse = Request.post(mockServer.getUrl() + "/form")
      .bodyForm(
        new BasicNameValuePair("id", UUID.randomUUID().toString()),
        new BasicNameValuePair("value", "3"),
        new BasicNameValuePair("value", "1"),
        new BasicNameValuePair("value", "2")).execute().returnResponse();
    assertThat(httpResponse.getCode(), is(equalTo(200)));
  }
}
