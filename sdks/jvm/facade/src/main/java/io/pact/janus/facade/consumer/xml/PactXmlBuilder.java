// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.xml;

import io.pact.janus.facade.consumer.dsl.BodyBuilder;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleCategory;
import io.pact.janus.facade.internal.ContentTypes;
import java.io.ByteArrayOutputStream;
import java.io.OutputStreamWriter;
import java.nio.charset.Charset;
import java.util.Map;
import java.util.function.Consumer;
import javax.xml.parsers.DocumentBuilder;
import javax.xml.parsers.DocumentBuilderFactory;
import javax.xml.parsers.ParserConfigurationException;
import javax.xml.transform.OutputKeys;
import javax.xml.transform.Transformer;
import javax.xml.transform.TransformerException;
import javax.xml.transform.TransformerFactory;
import javax.xml.transform.dom.DOMSource;
import javax.xml.transform.stream.StreamResult;
import org.w3c.dom.Document;
import org.w3c.dom.Element;

/** Builds an XML body, with rules at its elements, text and attributes. */
public class PactXmlBuilder implements BodyBuilder {
  private String rootName;
  private String rootNameSpace;
  private Map<String, String> namespaces;
  private String version;
  private String charset;
  private boolean standalone;
  private final Generators generators = new Generators();
  private final MatchingRuleCategory matchingRules = new MatchingRuleCategory("body");
  private Document doc;

  public PactXmlBuilder(String rootName) {
    this(rootName, null, Map.of(), null, null, false);
  }

  public PactXmlBuilder(String rootName, String rootNameSpace) {
    this(rootName, rootNameSpace, Map.of(), null, null, false);
  }

  public PactXmlBuilder(String rootName, String rootNameSpace, Map<String, String> namespaces) {
    this(rootName, rootNameSpace, namespaces, null, null, false);
  }

  public PactXmlBuilder(String rootName, String rootNameSpace, Map<String, String> namespaces, String version) {
    this(rootName, rootNameSpace, namespaces, version, null, false);
  }

  public PactXmlBuilder(String rootName, String rootNameSpace, Map<String, String> namespaces, String version,
      String charset) {
    this(rootName, rootNameSpace, namespaces, version, charset, false);
  }

  public PactXmlBuilder(String rootName, String rootNameSpace, Map<String, String> namespaces, String version,
      String charset, boolean standalone) {
    this.rootName = rootName;
    this.rootNameSpace = rootNameSpace;
    this.namespaces = namespaces;
    this.version = version;
    this.charset = charset;
    this.standalone = standalone;
  }

  public PactXmlBuilder build(Consumer<XmlNode> cl) {
    DocumentBuilder builder;
    try {
      builder = DocumentBuilderFactory.newInstance().newDocumentBuilder();
    } catch (ParserConfigurationException e) {
      throw new IllegalStateException(e);
    }
    doc = rootNameSpace != null
        ? builder.getDOMImplementation().createDocument(rootNameSpace, "ns:" + rootName, null)
        : builder.newDocument();
    if (version != null) {
      doc.setXmlVersion(version);
    }
    doc.setXmlStandalone(standalone);
    Element root = doc.getDocumentElement();
    if (root == null) {
      root = doc.createElement(rootName);
      doc.appendChild(root);
    }
    cl.accept(new XmlNode(this, root, java.util.List.of("$", qualifiedName(rootName))));
    return this;
  }

  public String qualifiedName(String name) {
    return rootNameSpace == null || rootNameSpace.isEmpty() ? name : "ns:" + name;
  }

  public Document getDoc() {
    return doc;
  }

  public byte[] asBytes() {
    return asBytes(null);
  }

  public byte[] asBytes(Charset charset) {
    try {
      Transformer transformer = TransformerFactory.newInstance().newTransformer();
      transformer.setOutputProperty(OutputKeys.INDENT, "yes");
      if (standalone) {
        transformer.setOutputProperty(OutputKeys.STANDALONE, "yes");
      }
      ByteArrayOutputStream out = new ByteArrayOutputStream();
      StreamResult result = charset != null ? new StreamResult(new OutputStreamWriter(out, charset)) : new StreamResult(out);
      transformer.transform(new DOMSource(doc), result);
      return out.toByteArray();
    } catch (TransformerException e) {
      throw new IllegalStateException(e);
    }
  }

  @Override
  public String toString() {
    return new String(asBytes());
  }

  public MatchingRuleCategory getMatchingRules() {
    return matchingRules;
  }

  @Override
  public MatchingRuleCategory getMatchers() {
    return matchingRules;
  }

  @Override
  public Generators getGenerators() {
    return generators;
  }

  @Override
  public String getContentType() {
    return charset == null || charset.isEmpty() ? "application/xml" : "application/xml; charset=" + charset;
  }

  @Override
  public byte[] buildBody() {
    return asBytes(ContentTypes.charset(getContentType()));
  }

  public PactXmlBuilder withRootName(String name) {
    this.rootName = name;
    return this;
  }

  public PactXmlBuilder withRootNameSpace(String nameSpace) {
    this.rootNameSpace = nameSpace;
    return this;
  }

  public PactXmlBuilder withNamespaces(Map<String, String> namespaces) {
    this.namespaces = namespaces;
    return this;
  }

  public PactXmlBuilder withVersion(String version) {
    this.version = version;
    return this;
  }

  public PactXmlBuilder withCharset(String charset) {
    this.charset = charset;
    return this;
  }

  public PactXmlBuilder withStandalone(boolean standalone) {
    this.standalone = standalone;
    return this;
  }
}
