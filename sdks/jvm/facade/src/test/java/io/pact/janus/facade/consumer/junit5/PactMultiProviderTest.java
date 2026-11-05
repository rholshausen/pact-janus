// pact-jvm's own test at 1928f2247 (4.7.8), consumer/junit5/src/test/java/.../PactMultiProviderTest.java, its imports
// changed from au.com.dius.pact. to io.pact.janus.facade. (staged plan S2.5b, ADR 0070).
package io.pact.janus.facade.consumer.junit5;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslRootValue;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit.MockServerConfig;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.V4Pact;
import io.pact.janus.facade.core.model.annotations.Pact;
import io.restassured.RestAssured;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;

import java.util.Map;

import static org.hamcrest.MatcherAssert.assertThat;
import static org.hamcrest.Matchers.is;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(pactVersion = PactSpecVersion.V4)
@MockServerConfig(providerName = "rest-heroes", port = PactMultiProviderTest.HEROES_PORT, hostInterface = "localhost")
@MockServerConfig(providerName = "rest-villains", port = PactMultiProviderTest.VILLAINS_PORT, hostInterface = "localhost")
public class PactMultiProviderTest {
	static final String HEROES_PORT = "1234";
	static final String VILLAINS_PORT = "1235";

	@Pact(consumer = "rest-fights", provider = "rest-heroes")
  public V4Pact helloHeroesPact(PactDslWithProvider builder) {
    return builder
      .uponReceiving("A hello request")
        .path("/hello/hero")
        .method("GET")
      .willRespondWith()
        .headers(Map.of("Content-Type", "text/plain"))
        .status(200)
        .body(PactDslRootValue.stringMatcher(".+", "Hello Heroes!"))
      .toPact(V4Pact.class);
  }

  @Pact(consumer = "rest-fights", provider = "rest-villains")
  public V4Pact helloVillainsPact(PactDslWithProvider builder) {
    return builder
      .uponReceiving("A hello request")
        .path("/hello/villain")
        .method("GET")
      .willRespondWith()
        .headers(Map.of("Content-Type", "text/plain"))
        .status(200)
        .body(PactDslRootValue.stringMatcher(".+", "Hello Villains!"))
      .toPact(V4Pact.class);
  }

	@Test
	@PactTestFor(pactMethods = { "helloHeroesPact", "helloVillainsPact" })
	public void helloHeroesAndVillains(@ForProvider("rest-heroes") MockServer heroesMockServer, @ForProvider("rest-villains") MockServer villainsMockServer) {
		assertThat(heroesMockServer.getPort(), is(Integer.parseInt(HEROES_PORT)));

		assertThat(villainsMockServer.getPort(), is(Integer.parseInt(VILLAINS_PORT)));

		testHelloHeroes();
		testHelloVillains();
	}

	@Test
	@PactTestFor(pactMethod = "helloHeroesPact")
	public void helloHeroes(@ForProvider("rest-heroes") MockServer mockServer) {
		assertThat(mockServer.getPort(), is(Integer.parseInt(HEROES_PORT)));

		testHelloHeroes();
	}

	@Test
	@PactTestFor(pactMethod = "helloVillainsPact")
	public void helloVillains(@ForProvider("rest-villains") MockServer mockServer) {
		assertThat(mockServer.getPort(), is(Integer.parseInt(VILLAINS_PORT)));

		testHelloVillains();
	}

	private void testHelloHeroes() {
		RestAssured.given()
			.port(Integer.parseInt(HEROES_PORT))
			.when().get("/hello/hero").then()
			.statusCode(200)
			.contentType("text/plain")
			.body(is("Hello Heroes!"));
	}

	private void testHelloVillains() {
		RestAssured.given()
			.port(Integer.parseInt(VILLAINS_PORT))
			.when().get("/hello/villain").then()
			.statusCode(200)
			.contentType("text/plain")
			.body(is("Hello Villains!"));
	}
}
