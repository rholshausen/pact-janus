// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer;

import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.core.model.Interaction;
import io.pact.janus.facade.core.model.PactSpecVersion;
import java.io.StringWriter;
import java.util.ArrayList;
import java.util.List;
import javax.xml.transform.OutputKeys;
import javax.xml.transform.Transformer;
import javax.xml.transform.TransformerException;
import javax.xml.transform.TransformerFactory;
import javax.xml.transform.dom.DOMSource;
import javax.xml.transform.stream.StreamResult;
import org.w3c.dom.Document;

/** Where a consumer's pact with a provider starts. */
public class ConsumerPactBuilder {
  private final String consumerName;
  private PactSpecVersion version = PactSpecVersion.V3;
  private final List<Interaction> interactions = new ArrayList<>();

  public ConsumerPactBuilder(String consumerName) {
    this.consumerName = consumerName;
  }

  public static ConsumerPactBuilder consumer(String consumer) {
    return new ConsumerPactBuilder(consumer);
  }

  public static PactDslJsonBody jsonBody() {
    return new PactDslJsonBody();
  }

  public static String xmlToString(Document body) throws TransformerException {
    Transformer transformer = TransformerFactory.newInstance().newTransformer();
    transformer.setOutputProperty(OutputKeys.INDENT, "yes");
    StringWriter writer = new StringWriter();
    transformer.transform(new DOMSource(body), new StreamResult(writer));
    return writer.toString();
  }

  public String getConsumerName() {
    return consumerName;
  }

  public List<Interaction> getInteractions() {
    return interactions;
  }

  public PactDslWithProvider hasPactWith(String provider) {
    return new PactDslWithProvider(this, provider, version);
  }

  public ConsumerPactBuilder pactSpecVersion(PactSpecVersion version) {
    this.version = version;
    return this;
  }
}
