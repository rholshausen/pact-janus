// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.xml;

import io.pact.janus.facade.consumer.dsl.Matcher;
import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.matchingrules.TypeMatcher;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;
import org.w3c.dom.Element;

/** One element of an XML body being built. */
public class XmlNode {
  private final PactXmlBuilder builder;
  private final Element element;
  private final List<String> path;

  public XmlNode(PactXmlBuilder builder, Element element, List<String> path) {
    this.builder = builder;
    this.element = element;
    this.path = path;
  }

  /** pact-jvm's {@code matcherKey}: the path and keys joined, an index joined without a dot. */
  private static String matcherKey(List<String> path, String... key) {
    List<String> all = new ArrayList<>(path);
    all.addAll(List.of(key));
    StringBuilder joined = new StringBuilder(all.get(0));
    for (String segment : all.subList(1, all.size())) {
      joined.append(segment.startsWith("[") ? "" : ".").append(segment);
    }
    return joined.toString();
  }

  private List<String> child(String name) {
    List<String> child = new ArrayList<>(path);
    child.add(name);
    return child;
  }

  public void setAttributes(Map<String, ?> attributes) {
    setElementAttributes(attributes, element);
  }

  public void eachLike(String name) {
    eachLike(name, 1, Map.of(), null);
  }

  public void eachLike(String name, int examples) {
    eachLike(name, examples, Map.of(), null);
  }

  public void eachLike(String name, int examples, Map<String, ?> attributes) {
    eachLike(name, examples, attributes, null);
  }

  public void eachLike(String name, int examples, Map<String, ?> attributes, Consumer<XmlNode> cl) {
    builder.getMatchingRules().addRule(matcherKey(path, name), TypeMatcher.INSTANCE);
    Element child = builder.getDoc().createElement(name);
    XmlNode node = new XmlNode(builder, child, child(child.getTagName()));
    node.setAttributes(attributes);
    if (cl != null) {
      cl.accept(node);
    }
    element.appendChild(child);
    for (int i = 2; i <= examples; i++) {
      element.appendChild(child.cloneNode(true));
    }
  }

  public void appendElement(String name) {
    appendElement(name, Map.of(), (Consumer<XmlNode>) null);
  }

  public void appendElement(String name, Map<String, ?> attributes) {
    appendElement(name, attributes, (Consumer<XmlNode>) null);
  }

  public void appendElement(String name, Map<String, ?> attributes, Consumer<XmlNode> cl) {
    Element child = builder.getDoc().createElement(name);
    XmlNode node = new XmlNode(builder, child, child(child.getTagName()));
    node.setAttributes(attributes);
    if (cl != null) {
      cl.accept(node);
    }
    element.appendChild(child);
  }

  public void appendElement(String name, Map<String, ?> attributes, String contents) {
    Element child = builder.getDoc().createElement(name);
    XmlNode node = new XmlNode(builder, child, child(child.getTagName()));
    node.setAttributes(attributes);
    child.setTextContent(contents);
    element.appendChild(child);
  }

  public void appendElement(String name, Map<String, ?> attributes, Matcher contents) {
    Element child = builder.getDoc().createElement(name);
    setElementAttributes(attributes, child);
    child.setTextContent(String.valueOf(contents.getValue()));
    builder.getMatchingRules().addRule(matcherKey(path, name, "#text"), contents.getMatcher());
    if (contents.getGenerator() != null) {
      builder.getGenerators().addGenerator(Category.BODY, matcherKey(path, name, "#text"), contents.getGenerator());
    }
    element.appendChild(child);
  }

  private void setElementAttributes(Map<String, ?> attributes, Element target) {
    attributes.forEach((key, value) -> {
      if (value instanceof Matcher matcher) {
        builder.getMatchingRules().addRule(matcherKey(path, "['@" + key + "']"), matcher.getMatcher());
        if (matcher.getGenerator() != null) {
          builder.getGenerators().addGenerator(Category.BODY, matcherKey(path, "['@" + key + "']"), matcher.getGenerator());
        }
        target.setAttribute(key, String.valueOf(matcher.getValue()));
      } else {
        target.setAttribute(key, String.valueOf(value));
      }
    });
  }
}
