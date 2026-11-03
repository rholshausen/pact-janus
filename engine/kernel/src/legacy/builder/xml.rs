//! An XML body written as the FFI's XML builder document (ADR 0066 decision 2): `{ root: { name,
//! attributes, children } }`, the document pact-js's `XmlBuilder` builds, with matchers on elements,
//! attributes and text, turned into the XML text and the body's rules.
//!
//! Ported from pact-reference's `pact_ffi` at `1f1c2e61` (`src/mock_server/xml.rs`:
//! `generate_xml_body`, `create_element_from_json`, `add_attributes`, `duplicate_element`). Where the
//! FFI panics on a document it cannot read, this returns the problem: a panic must not cross the
//! engine boundary (AGENTS.md's architecture rules).

use super::integration::matchers_from_integration_json;
use pact_models::generators::{Generator, GeneratorCategory, Generators};
use pact_models::json_utils::json_to_string;
use pact_models::matchingrules::{MatchingRuleCategory, RuleLogic};
use pact_models::path_exp::DocPath;
use serde_json::{Map, Value};
use sxd_document::Package;
use sxd_document::dom::{ChildOfElement, Document, Element, Text};
use sxd_document::writer::format_document;

/// The XML text of an XML builder document, its matchers added to `rules` and `generators`.
pub fn generate_xml_body(
  document: &Map<String, Value>,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
) -> Result<Vec<u8>, String> {
  let package = Package::new();
  let doc = package.as_document();
  match document.get("root") {
    Some(Value::Object(root)) => {
      match create_element(doc, None, root, rules, generators, &["$".to_string()])? {
        Node::Element(element) => doc.root().append_child(element),
        Node::Text(_) => return Err("the XML builder document's root is text, not an element".to_string()),
      }
    }
    Some(other) => {
      return Err(format!(
        "the XML builder document's root is not an object: {other}"
      ));
    }
    None => return Err("the XML builder document has no root".to_string()),
  }
  let mut output = vec![];
  format_document(&doc, &mut output).map_err(|err| format!("the XML document cannot be written: {err}"))?;
  Ok(output)
}

enum Node<'a> {
  Element(Element<'a>),
  Text(Text<'a>),
}

fn create_element<'a>(
  doc: Document<'a>,
  parent: Option<Element<'a>>,
  object: &Map<String, Value>,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  path: &[String],
) -> Result<Node<'a>, String> {
  let node = if object.contains_key("pact:matcher:type") {
    // A matcher on an element: its value is the element.
    let Some(value) = object.get("value") else {
      return Err(format!("an XML matcher at '{}' has no value", path.join(".")));
    };
    let Value::Object(element) = value else {
      return Err(format!(
        "an XML matcher's value at '{}' is not an element: {value}",
        path.join(".")
      ));
    };
    let name = element.get("name").map(json_to_string).unwrap_or_default();
    let mut element_path = path.to_vec();
    element_path.push(name.clone());
    let at = DocPath::new(element_path.join(".")).unwrap_or(DocPath::root());
    add_matcher(object, object, rules, generators, &at, path)?;
    let new_element = doc.create_element(name.as_str());
    if let Some(Value::Object(attributes)) = element.get("attributes") {
      add_attributes(&new_element, attributes, rules, generators, &element_path)?;
    }
    add_children(
      doc,
      new_element,
      element.get("children"),
      rules,
      generators,
      &element_path,
    )?;
    Node::Element(new_element)
  } else if let Some(content) = object.get("content") {
    let text = doc.create_text(json_to_string(content).as_str());
    if let Some(matcher) = object.get("matcher") {
      let mut text_path = path.to_vec();
      text_path.push("#text".to_string());
      let at = DocPath::new(text_path.join(".")).unwrap_or(DocPath::root());
      if let Value::Object(matcher) = matcher {
        add_matcher(matcher, object, rules, generators, &at, path)?;
      }
    }
    Node::Text(text)
  } else if let Some(name) = object.get("name") {
    let name = json_to_string(name);
    let new_element = doc.create_element(name.as_str());
    let mut element_path = path.to_vec();
    element_path.push(name);
    if let Some(Value::Object(attributes)) = object.get("attributes") {
      add_attributes(&new_element, attributes, rules, generators, &element_path)?;
    }
    add_children(
      doc,
      new_element,
      object.get("children"),
      rules,
      generators,
      &element_path,
    )?;
    Node::Element(new_element)
  } else {
    return Err(format!(
      "an XML builder node at '{}' is neither an element, a matcher nor text: {}",
      path.join("."),
      Value::Object(object.clone())
    ));
  };

  if let Some(parent) = parent {
    match &node {
      Node::Element(element) => {
        // A matcher's `examples` is how many copies of the element the example holds.
        let examples = object.get("examples").and_then(Value::as_u64).unwrap_or(1);
        for _ in 0..examples {
          parent.append_child(duplicate_element(doc, element));
        }
      }
      Node::Text(text) => parent.append_child(*text),
    }
  }
  Ok(node)
}

/// A matcher's rules and generator at `at`. The generator a `pact:generator:type` names reads its
/// configuration from `generator_source`: the matcher for an element, the text node for text.
fn add_matcher(
  matcher: &Map<String, Value>,
  generator_source: &Map<String, Value>,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  at: &DocPath,
  path: &[String],
) -> Result<(), String> {
  let (matched, generator) = matchers_from_integration_json(matcher)
    .map_err(|err| format!("the XML matcher at '{}' cannot be read: {err}", path.join(".")))?;
  for rule in matched {
    rules.add_rule(at.clone(), rule, RuleLogic::And);
  }
  if let Some(generator) = generator {
    generators.add_generator_with_subcategory(&GeneratorCategory::BODY, at.clone(), generator);
  }
  if let Some(generator_type) = generator_source.get("pact:generator:type")
    && let Some(generator) = Generator::from_map(&json_to_string(generator_type), matcher)
  {
    generators.add_generator_with_subcategory(&GeneratorCategory::BODY, at.clone(), generator);
  }
  Ok(())
}

fn add_children<'a>(
  doc: Document<'a>,
  element: Element<'a>,
  children: Option<&Value>,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  path: &[String],
) -> Result<(), String> {
  match children {
    None => Ok(()),
    Some(Value::Array(children)) => {
      for child in children {
        let Value::Object(child) = child else {
          return Err(format!(
            "a child of '{}' is not an object: {child}",
            path.join(".")
          ));
        };
        create_element(doc, Some(element), child, rules, generators, path)?;
      }
      Ok(())
    }
    Some(other) => Err(format!(
      "the children of '{}' are not a list: {other}",
      path.join(".")
    )),
  }
}

fn add_attributes(
  element: &Element,
  attributes: &Map<String, Value>,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  path: &[String],
) -> Result<(), String> {
  for (name, value) in attributes {
    let at = format!("{}['@{name}']", path.join("."));
    let text = match value {
      Value::Object(matcher) if matcher.contains_key("pact:matcher:type") => {
        let doc_path = DocPath::new(at.clone()).unwrap_or(DocPath::root());
        let (matched, generator) = matchers_from_integration_json(matcher)
          .map_err(|err| format!("the XML matcher at '{at}' cannot be read: {err}"))?;
        for rule in matched {
          rules.add_rule(doc_path.clone(), rule, RuleLogic::And);
        }
        if let Some(generator) = generator {
          generators.add_generator_with_subcategory(&GeneratorCategory::BODY, doc_path.clone(), generator);
        }
        if let Some(generator_type) = matcher.get("pact:generator:type")
          && let Some(generator) = Generator::from_map(&json_to_string(generator_type), matcher)
        {
          generators.add_generator_with_subcategory(&GeneratorCategory::BODY, doc_path, generator);
        }
        matcher.get("value").map(json_to_string).unwrap_or_default()
      }
      _ => json_to_string(value),
    };
    element.set_attribute_value(name.as_str(), text.as_str());
  }
  Ok(())
}

fn duplicate_element<'a>(doc: Document<'a>, element: &Element<'a>) -> Element<'a> {
  let copy = doc.create_element(element.name());
  for attribute in element.attributes() {
    copy.set_attribute_value(attribute.name(), attribute.value());
  }
  for child in element.children() {
    match child {
      ChildOfElement::Element(child) => copy.append_child(duplicate_element(doc, &child)),
      ChildOfElement::Text(text) => copy.append_child(doc.create_text(text.text())),
      _ => {}
    }
  }
  copy
}
