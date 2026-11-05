// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../MultiProviderWithStaticPortsTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit.MockServerConfig;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.pact.janus.facade.core.support.Response;
import io.pact.janus.facade.core.support.SimpleHttp;
import groovy.json.JsonOutput;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.util.Map;

import static io.pact.janus.facade.consumer.dsl.LambdaDsl.newJsonBody;
import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.is;
import static org.hamcrest.Matchers.notNullValue;

@SuppressWarnings("UnusedMethodParameter")
@ExtendWith(PactConsumerTestExt.class)
@MockServerConfig(providerName = "provider1", port = "1234")
@MockServerConfig(providerName = "provider2", port = "1235")
class MultiProviderWithStaticPortsTest {

  @Pact(provider = "provider1", consumer= "consumer")
  RequestResponsePact pact1(PactDslWithProvider builder) {
    return builder
      .uponReceiving("a new user request")
        .path("/users")
        .method("POST")
        .body(newJsonBody(body -> body.stringType("name", "bob")).build())
      .willRespondWith()
        .status(201)
        .matchHeader("Location", "http(s)?://\\w+:\\d{4}/user/\\d{16}")
      .toPact();
  }

  @Pact(provider = "provider2", consumer= "consumer")
  RequestResponsePact pact2(PactDslWithProvider builder) {
    return builder
      .uponReceiving("a new user")
      .path("/users")
      .method("POST")
      .body(newJsonBody(body -> body.numberType("id", 2047176700442987L)).build())
      .willRespondWith()
      .status(204)
      .toPact();
  }

  @Test
  @PactTestFor(pactMethods = {"pact1", "pact2"}, pactVersion = PactSpecVersion.V3)
  void runTest(@ForProvider("provider1") MockServer mockServer1, @ForProvider("provider2") MockServer mockServer2) {
    assertThat(mockServer1.getPort(), is(1234));
    assertThat(mockServer2.getPort(), is(1235));

    SimpleHttp http = new SimpleHttp(mockServer1.getUrl());

    Response response = http.post("/users", JsonOutput.toJson(Map.of("name", "Fred")),
      "application/json; charset=UTF-8");
    assertThat(response.getStatusCode(), is(201));

    String value = response.getHeaders().get("location").get(0);
    assertThat(value, is(notNullValue()));
    String[] strings = value.split("/");
    String id = strings[strings.length - 1];

    SimpleHttp http2 = new SimpleHttp(mockServer2.getUrl());
    Response response2 = http2.post("/users", JsonOutput.toJson(Map.of("id", Long.parseLong(id))),
      "application/json; charset=UTF-8");
    assertThat(response2.getStatusCode(), is(204));
  }
}
