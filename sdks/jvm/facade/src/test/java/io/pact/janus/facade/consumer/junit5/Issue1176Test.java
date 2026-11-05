// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../Issue1176Test.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit.MockServerConfig;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.restassured.builder.RequestSpecBuilder;
import io.restassured.filter.log.RequestLoggingFilter;
import io.restassured.filter.log.ResponseLoggingFilter;
import io.restassured.http.ContentType;
import io.restassured.response.Response;
import io.restassured.specification.RequestSpecification;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.util.Collections;
import java.util.Map;

import static io.restassured.RestAssured.given;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;

@ExtendWith(PactConsumerTestExt.class)
public class Issue1176Test {
  private static final String CONFIG_URL = "/config";

  @BeforeEach
  public void setUp(MockServer mockServer) {
    assertNotNull(mockServer);
  }

  @Pact(provider = "config-service", consumer = "test-integration")
  public RequestResponsePact validCredentials(PactDslWithProvider builder) {
    Map<String, String> headers = Collections.singletonMap("Content-Type", ContentType.TEXT.toString());

    RequestResponsePact pact = builder
      .uponReceiving("valid configuration")
      .path(CONFIG_URL)
      .method("GET")
      .headers(headers)
      .body("text")
      .willRespondWith()
      .status(200)
      .body("{\"data\":\"\", \"status\":\"success\"}")
      .toPact();

    return pact;
  }

  @Test
  @PactTestFor(pactMethod = "validCredentials", pactVersion = PactSpecVersion.V3)
  @MockServerConfig(hostInterface = "localhost", port = "7001")
  public void runTest(MockServer mockServer) {
    RequestLoggingFilter requestLoggingFilter = new RequestLoggingFilter();
    ResponseLoggingFilter responseLoggingFilter = new ResponseLoggingFilter();

    RequestSpecification requestSpec = new RequestSpecBuilder()
      .setContentType(ContentType.TEXT)
      .setPort(mockServer.getPort())
      .setBasePath(CONFIG_URL)
      .addFilter(requestLoggingFilter)
      .addFilter(responseLoggingFilter)
      .setBody("text")
      .build();

    Response response = given().spec(requestSpec).get();
    assertEquals("success", response.body().jsonPath().get("status"));
  }
}
