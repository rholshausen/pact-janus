//! A v1–v4 XML body as a document (ADR 0039 decision 3): parsed as today's verifier parses one
//! (`pact_models::xml_utils::parse_bytes`, which is `sxd-document`), into the model the built-in XML
//! content component, `engine/component-xml`, decodes a reply to. The pact's side and the reply's
//! side must agree on that model for a verdict to mean anything, so `tests/xml_bodies.rs` holds this
//! reader to the component's.
//!
//! The model, which the component's module docs give in full: an object with one member, the root
//! element's local name, holding the root element. An element is an object of `#namespace` (when it
//! is in one), `@<name>` per attribute (Clark notation, `@{<uri>}<local>`, when namespaced, in name
//! order), `#text` (its text as written, when not empty and not only indentation between child
//! elements), `#trimmed` (its text nodes trimmed and joined, when not empty: today's verifier's
//! reading, which the v1–v4 compiler compares; ADR 0075), and an array of child elements per local
//! name, in the order the names first appear.

use crate::plan::RuntimeValue;
use indexmap::IndexMap;
use sxd_document::dom::{ChildOfElement, Element};

/// The member holding an element's namespace URI.
pub const NAMESPACE: &str = "#namespace";

/// The member holding an element's text, as written.
pub const TEXT: &str = "#text";

/// The member holding an element's text as today's verifier reads it: each text node trimmed, then
/// joined. What the v1–v4 compiler and `janus upgrade` compare.
pub const TRIMMED: &str = "#trimmed";

/// The namespace namespace declarations are in.
const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

/// `body` as an XML document (module docs), or why it is not one.
pub fn document(body: &[u8]) -> Result<RuntimeValue, String> {
  let package = pact_models::xml_utils::parse_bytes(body).map_err(|err| err.to_string())?;
  let document = package.as_document();
  let root = document
    .root()
    .children()
    .into_iter()
    .find_map(|child| child.element())
    .ok_or_else(|| "an XML document needs a root element".to_string())?;
  Ok(RuntimeValue::Object(IndexMap::from([(
    root.name().local_part().to_string(),
    element(root),
  )])))
}

fn element(element: Element<'_>) -> RuntimeValue {
  let mut members: IndexMap<String, RuntimeValue> = IndexMap::new();
  if let Some(namespace) = element.name().namespace_uri() {
    members.insert(NAMESPACE.to_string(), RuntimeValue::String(namespace.to_string()));
  }
  let mut attributes: Vec<(String, String)> = element
    .attributes()
    .into_iter()
    .filter(|attribute| {
      let name = attribute.name();
      name.namespace_uri() != Some(XMLNS_NAMESPACE)
        && !(name.namespace_uri().is_none() && name.local_part() == "xmlns")
    })
    .map(|attribute| {
      let name = attribute.name();
      let name = match name.namespace_uri() {
        Some(namespace) => format!("@{{{namespace}}}{}", name.local_part()),
        None => format!("@{}", name.local_part()),
      };
      (name, attribute.value().to_string())
    })
    .collect();
  attributes.sort();
  for (name, value) in attributes {
    members.insert(name, RuntimeValue::String(value));
  }
  let children = element.children();
  let nodes: Vec<&str> = children
    .iter()
    .filter_map(|child| match child {
      ChildOfElement::Text(text) => Some(text.text()),
      _ => None,
    })
    .collect();
  let text = nodes.concat();
  let has_elements = children
    .iter()
    .any(|child| matches!(child, ChildOfElement::Element(_)));
  if !text.is_empty() && !(has_elements && text.trim().is_empty()) {
    members.insert(TEXT.to_string(), RuntimeValue::String(text));
  }
  let trimmed: String = nodes.iter().map(|node| node.trim()).collect();
  if !trimmed.is_empty() {
    members.insert(TRIMMED.to_string(), RuntimeValue::String(trimmed));
  }
  for child in children.into_iter().filter_map(|child| child.element()) {
    let entry = members
      .entry(child.name().local_part().to_string())
      .or_insert_with(|| RuntimeValue::Array(Vec::new()));
    if let RuntimeValue::Array(items) = entry {
      items.push(self::element(child));
    }
  }
  RuntimeValue::Object(members)
}

/// An element's or attribute's name as a v1–v4 rule path names it: `<uri>:<local>` when it is in a
/// namespace (`pact_matching`'s `name`), its local name otherwise. `name` is the member's key, in
/// Clark notation for an attribute; an element's namespace is its own `#namespace`.
pub fn rule_name(name: &str, namespace: Option<&str>) -> String {
  let (namespace, local) = match name.strip_prefix('{').and_then(|rest| rest.split_once('}')) {
    Some((namespace, local)) => (Some(namespace), local),
    None => (namespace, name),
  };
  match namespace {
    Some(namespace) => format!("{namespace}:{local}"),
    None => local.to_string(),
  }
}
