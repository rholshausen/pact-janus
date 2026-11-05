// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../Ip6Test.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit.MockServerConfig;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import org.apache.hc.client5.http.fluent.Request;
import org.apache.hc.core5.http.ClassicHttpResponse;
import org.hamcrest.CoreMatchers;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.DisabledOnOs;
import org.junit.jupiter.api.condition.OS;
import org.junit.jupiter.api.extension.ExtendWith;

import java.io.IOException;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.equalTo;
import static org.hamcrest.Matchers.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "ip6_provider", pactVersion = PactSpecVersion.V3)
@MockServerConfig(hostInterface = "::1", port = "1234")
@DisabledOnOs(OS.WINDOWS)
public class Ip6Test {
  @Pact(consumer = "ApiConsumer")
  public RequestResponsePact articles(PactDslWithProvider builder) {
    return builder
            .uponReceiving("GET request")
                    .path("/test")
                    .method("GET")
                    .willRespondWith()
                    .status(200)
                    .toPact();
  }

  @Test
  @PactTestFor
  void testApi(MockServer mockServer) throws IOException {
    ClassicHttpResponse httpResponse = (ClassicHttpResponse) Request.get(mockServer.getUrl() + "/test").execute().returnResponse();
    assertThat(httpResponse.getCode(), is(equalTo(200)));
  }
}
