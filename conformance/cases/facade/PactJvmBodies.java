// facade/pact-jvm-bodies: each body pact-jvm's builders take — a DslPart, a String with and without a
// content type, single quotes, an XML Document, a form, a PactXmlBuilder, binary data, and a body
// matched by content type — each served as the test sends it.
import static org.junit.jupiter.api.Assertions.assertEquals;

import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.FormPostBuilder;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.consumer.xml.PactXmlBuilder;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import javax.xml.parsers.DocumentBuilderFactory;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.w3c.dom.Document;
import org.w3c.dom.Element;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "orders-api")
class PactJvmBodies {
  @Pact(consumer = "web-app")
  RequestResponsePact bodies(PactDslWithProvider builder) throws Exception {
    Document document = DocumentBuilderFactory.newInstance().newDocumentBuilder().newDocument();
    Element order = document.createElement("order");
    order.setAttribute("id", "1");
    document.appendChild(order);
    return builder
        .uponReceiving("post a DslPart").path("/dsl").method("POST")
        .body(new PactDslJsonBody().stringType("sku", "A-1"))
        .willRespondWith().status(201)
        .uponReceiving("post a JSON string").path("/json").method("POST")
        .body("{\"sku\": \"A-1\"}")
        .willRespondWith().status(201)
        .uponReceiving("post text with its type").path("/text").method("POST")
        .body("sku=A-1", "text/plain")
        .willRespondWith().status(201)
        .uponReceiving("post JSON in single quotes").path("/quoted").method("POST")
        .bodyWithSingleQuotes("{'sku': 'A-1'}")
        .willRespondWith().status(201)
        .uponReceiving("post an XML document").path("/document").method("POST")
        .body(document)
        .willRespondWith().status(201)
        .uponReceiving("post a form").path("/form").method("POST")
        .body(new FormPostBuilder().stringMatcher("sku", "[A-Z]-\\d+", "A-1").stringValue("quantity", "2"))
        .willRespondWith().status(201)
        .uponReceiving("post a built XML body").path("/xml").method("POST")
        .body(new PactXmlBuilder("order").build(root -> root.appendElement("sku", Map.of(), "A-1")))
        .willRespondWith().status(201)
        .uponReceiving("post binary data").path("/binary").method("POST")
        .withBinaryData(new byte[] {1, 2, 3}, "application/octet-stream")
        .willRespondWith().status(201)
        .uponReceiving("get JSON of its content type").path("/typed").method("GET")
        .willRespondWith().status(200)
        .bodyMatchingContentType("application/json", "{\"sku\": \"A-1\"}")
        .toPact();
  }

  @Test
  void sendsEachBody(MockServer mockServer) throws Exception {
    String url = mockServer.getUrl();
    // pact-jvm's builder declares the charset of a JSON body, which its mock requires too.
    assertEquals(201, post(url + "/dsl", "application/json; charset=UTF-8", "{\"sku\": \"B-2\"}"));
    assertEquals(201, post(url + "/json", "application/json", "{\"sku\": \"A-1\"}"));
    assertEquals(201, post(url + "/text", "text/plain", "sku=A-1"));
    assertEquals(201, post(url + "/quoted", "application/json", "{\"sku\": \"A-1\"}"));
    // As pact-jvm's: an XML Document is HttpCore's application/xml, whose charset is ISO-8859-1.
    assertEquals(201, post(url + "/document", "application/xml; charset=ISO-8859-1", "<order id=\"1\"/>"));
    assertEquals(201, post(url + "/form", "application/x-www-form-urlencoded; charset=ISO-8859-1", "sku=Q-9&quantity=2"));
    assertEquals(201, post(url + "/xml", "application/xml; charset=ISO-8859-1", "<order><sku>A-1</sku></order>"));
    assertEquals(201, HttpClient.newHttpClient().send(HttpRequest.newBuilder(URI.create(url + "/binary"))
        .header("Content-Type", "application/octet-stream").POST(HttpRequest.BodyPublishers.ofByteArray(new byte[] {1, 2, 3}))
        .build(), HttpResponse.BodyHandlers.discarding()).statusCode());
    HttpResponse<String> typed = HttpClient.newHttpClient().send(HttpRequest.newBuilder(URI.create(url + "/typed")).GET()
        .build(), HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));
    assertEquals("{\"sku\": \"A-1\"}", typed.body());
  }

  private static int post(String url, String contentType, String body) throws Exception {
    HttpResponse<String> response = HttpClient.newHttpClient().send(HttpRequest.newBuilder(URI.create(url))
        .header("Content-Type", contentType).POST(HttpRequest.BodyPublishers.ofString(body)).build(),
        HttpResponse.BodyHandlers.ofString());
    if (response.statusCode() != 201) {
      throw new AssertionError(url + ": " + response.statusCode() + " " + response.body());
    }
    return response.statusCode();
  }
}
