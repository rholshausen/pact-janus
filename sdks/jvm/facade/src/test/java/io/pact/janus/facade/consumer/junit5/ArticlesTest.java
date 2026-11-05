// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../ArticlesTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
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
import static org.hamcrest.Matchers.notNullValue;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "ArticlesProvider", pactVersion = PactSpecVersion.V3)
@MockServerConfig(hostInterface = "localhost", port = "1234")
public class ArticlesTest {
  private Map<String, String> headers = MapUtils.putAll(new HashMap<>(), new String[] {
    "Content-Type", "application/json"
  });

  @BeforeEach
  public void setUp(MockServer mockServer) {
    assertThat(mockServer, is(notNullValue()));
  }

  @Pact(consumer = "ArticlesConsumer")
  public RequestResponsePact articles(PactDslWithProvider builder) {
    return builder
      .given("Articles exist")
      .uponReceiving("retrieving article data")
        .path("/articles.json")
        .method("GET")
      .willRespondWith()
        .headers(headers)
        .status(200)
        .body(
          new PactDslJsonBody()
            .minArrayLike("articles", 1)
              .object("variants")
                .eachKeyLike("0032")
                  .stringType("description", "sample description")
                  .closeObject()
                .closeObject()
              .closeObject()
            .closeArray()
        )
      .toPact();
  }

  @Pact(consumer = "ArticlesConsumer")
  public RequestResponsePact articlesDoNotExist(PactDslWithProvider builder) {
    return builder
      .given("No articles exist")
      .uponReceiving("retrieving article data")
      .path("/articles.json")
      .method("GET")
      .willRespondWith()
      .headers(headers)
      .status(404)
      .toPact();
  }

  @Test
  @PactTestFor(pactMethod = "articles")
  void testArticles(MockServer mockServer) throws IOException {
    ClassicHttpResponse httpResponse = (ClassicHttpResponse) Request.get(mockServer.getUrl() + "/articles.json").execute().returnResponse();
    assertThat(httpResponse.getCode(), is(equalTo(200)));
    assertThat(IOUtils.toString(httpResponse.getEntity().getContent()),
      is(equalTo("{\"articles\":[{\"variants\":{\"0032\":{\"description\":\"sample description\"}}}]}")));
  }

  @Test
  @PactTestFor(pactMethod = "articlesDoNotExist")
  void testArticlesDoNotExist(MockServer mockServer) throws IOException {
    ClassicHttpResponse httpResponse = (ClassicHttpResponse) Request.get(mockServer.getUrl() + "/articles.json").execute().returnResponse();
    assertThat(httpResponse.getCode(), is(equalTo(404)));
    assertThat(IOUtils.toString(httpResponse.getEntity().getContent()), is(equalTo("")));
  }
}
