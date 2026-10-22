//! The built-in XML content component (component-interfaces spec §6, ADR 0039): `application/xml`,
//! `text/xml` and every `+xml` type, octets to a document and back. In-tree, but bound through the
//! same interface a third-party component uses (ADR 0012), as the JSON, text and form ones are.
//!
//! An XML body's document (ADR 0039 decision 2) is an object with one member, the root element's
//! local name, whose value is the root element. An element is an object whose members are, in order:
//!
//! - `#namespace`, the element's namespace URI, when it is in one;
//! - `@<name>`, each attribute's value, in name order, since XML gives attributes no order and the
//!   parser keeps none. A namespaced attribute is named in Clark notation, `@{<uri>}<local>`.
//!   Namespace declarations are not attributes;
//! - `#text`, its text: each text node trimmed, then joined, as today's verifier reads it. Absent
//!   when that is empty;
//! - `<local name>`, an array of the child elements with that local name, in document order, one
//!   member per name in the order the name first appears.
//!
//! Comments, processing instructions, the prolog and namespace prefixes are not kept. Parsed with
//! `sxd-document`, as today's verifier parses one.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use indexmap::IndexMap;
use pact_janus_kernel::component::{
  Compile, CompileResult, ComponentError, ContentComponent, Decode, DecodeResult, Detect, DetectResult,
  Encode, EncodeResult, SlotValue,
};
use pact_janus_kernel::plan::RuntimeValue;
use serde_json::Value;
use sxd_document::dom::{ChildOfElement, Element};

/// The media type this component writes when it is asked for none in particular.
pub const MEDIA_TYPE: &str = "application/xml";

/// The member holding an element's namespace URI.
pub const NAMESPACE: &str = "#namespace";

/// The member holding an element's text.
pub const TEXT: &str = "#text";

/// The namespace the `xml` prefix is bound to, which is never declared.
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// The namespace namespace declarations are in.
const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

/// XML bodies: `application/xml`, `text/xml` and `+xml` types.
pub struct XmlContent;

impl XmlContent {
  pub fn new() -> Self {
    XmlContent
  }
}

impl Default for XmlContent {
  fn default() -> Self {
    Self::new()
  }
}

/// Whether `content_type` is an XML type, as today's verifier counts one (`ContentType::is_xml`): an
/// `application` or `text` type whose subtype is `xml` or ends in `+xml`. Parameters and case are
/// ignored.
pub fn is_xml(content_type: &str) -> bool {
  let essence = content_type
    .split(';')
    .next()
    .unwrap_or_default()
    .trim()
    .to_ascii_lowercase();
  let Some((main_type, subtype)) = essence.split_once('/') else {
    return false;
  };
  matches!(main_type, "application" | "text") && (subtype == "xml" || subtype.ends_with("+xml"))
}

/// `body` as an XML document (module docs). It must be UTF-8 and well formed, with a root element.
pub fn parse(body: &[u8]) -> Result<RuntimeValue, String> {
  let text = std::str::from_utf8(body).map_err(|err| format!("an XML body must be UTF-8: {err}"))?;
  let package = sxd_document::parser::parse(text).map_err(|err| format!("not well-formed XML: {err}"))?;
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
      (
        format!("@{}", clark(name.namespace_uri(), name.local_part())),
        attribute.value().to_string(),
      )
    })
    .collect();
  attributes.sort();
  for (name, value) in attributes {
    members.insert(name, RuntimeValue::String(value));
  }
  let children = element.children();
  let text: String = children
    .iter()
    .filter_map(|child| match child {
      ChildOfElement::Text(text) => Some(text.text().trim()),
      _ => None,
    })
    .collect();
  if !text.is_empty() {
    members.insert(TEXT.to_string(), RuntimeValue::String(text));
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

/// A name in Clark notation: `{<uri>}<local>`, or the local name alone when it is in no namespace.
pub fn clark(namespace: Option<&str>, local: &str) -> String {
  match namespace {
    Some(namespace) => format!("{{{namespace}}}{local}"),
    None => local.to_string(),
  }
}

/// A name in Clark notation split into its namespace and its local name.
pub fn split_clark(name: &str) -> (Option<&str>, &str) {
  match name.strip_prefix('{').and_then(|rest| rest.split_once('}')) {
    Some((namespace, local)) => (Some(namespace), local),
    None => (None, name),
  }
}

/// An XML document (module docs) as a body. The root element and every element in a namespace
/// declare it as the default namespace where it changes, a namespaced attribute's namespace gets a
/// generated prefix, and an element's text comes before its children.
pub fn write(document: &RuntimeValue) -> Result<String, String> {
  let RuntimeValue::Object(members) = document else {
    return Err("an XML document is an object with one member, its root element".to_string());
  };
  let mut roots = members.iter();
  let (Some((name, root)), None) = (roots.next(), roots.next()) else {
    return Err(format!(
      "an XML document has one root element, and this one has {}",
      members.len()
    ));
  };
  let root = match root {
    RuntimeValue::Array(items) if items.len() == 1 => &items[0],
    other => other,
  };
  let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
  write_element(&mut out, name, root, None)?;
  Ok(out)
}

fn write_element(
  out: &mut String,
  name: &str,
  element: &RuntimeValue,
  default_namespace: Option<&str>,
) -> Result<(), String> {
  let RuntimeValue::Object(members) = element else {
    return Err(format!("the element '{name}' must be an object"));
  };
  check_name(name)?;
  let namespace = match members.get(NAMESPACE) {
    None => None,
    Some(RuntimeValue::String(namespace)) => Some(namespace.as_str()),
    Some(_) => return Err(format!("the namespace of '{name}' must be a string")),
  };
  out.push('<');
  out.push_str(name);
  if namespace != default_namespace {
    out.push_str(&format!(" xmlns=\"{}\"", escape(namespace.unwrap_or_default())));
  }

  let mut prefixes: Vec<&str> = Vec::new();
  let mut attributes = String::new();
  let mut text = None;
  let mut children = Vec::new();
  for (key, value) in members {
    if let Some(attribute) = key.strip_prefix('@') {
      let (attribute_namespace, local) = split_clark(attribute);
      check_name(local)?;
      let qualified = match attribute_namespace {
        None => local.to_string(),
        Some(XML_NAMESPACE) => format!("xml:{local}"),
        Some(uri) => {
          let index = match prefixes.iter().position(|known| *known == uri) {
            Some(index) => index,
            None => {
              prefixes.push(uri);
              prefixes.len() - 1
            }
          };
          format!("ns{index}:{local}")
        }
      };
      let value =
        scalar_text(value).ok_or_else(|| format!("the attribute '{attribute}' of '{name}' must be text"))?;
      attributes.push_str(&format!(" {qualified}=\"{}\"", escape(&value)));
    } else if key == TEXT {
      text = Some(scalar_text(value).ok_or_else(|| format!("the text of '{name}' must be text"))?);
    } else if key == NAMESPACE {
      continue;
    } else if key.starts_with('#') {
      return Err(format!("'{key}' is not a member an XML element has"));
    } else {
      children.push((key, value));
    }
  }
  for (index, uri) in prefixes.iter().enumerate() {
    out.push_str(&format!(" xmlns:ns{index}=\"{}\"", escape(uri)));
  }
  out.push_str(&attributes);

  if text.is_none() && children.is_empty() {
    out.push_str("/>");
    return Ok(());
  }
  out.push('>');
  if let Some(text) = text {
    out.push_str(&escape(&text));
  }
  for (child_name, value) in children {
    match value {
      RuntimeValue::Array(items) => {
        for item in items {
          write_element(out, child_name, item, namespace)?;
        }
      }
      other => write_element(out, child_name, other, namespace)?,
    }
  }
  out.push_str(&format!("</{name}>"));
  Ok(())
}

/// A name an element or attribute can have: not empty, and none of what would end it early.
fn check_name(name: &str) -> Result<(), String> {
  let invalid =
    |c: char| c.is_whitespace() || matches!(c, '<' | '>' | '&' | '"' | '\'' | '/' | '=' | '{' | '}');
  if name.is_empty()
    || name.contains(invalid)
    || name.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.')
  {
    Err(format!("'{name}' is not an XML name"))
  } else {
    Ok(())
  }
}

/// A scalar as the text an attribute or an element's text carries: a string as it is, a number or
/// a boolean as written, null as nothing.
fn scalar_text(value: &RuntimeValue) -> Option<String> {
  match value {
    RuntimeValue::String(text) => Some(text.clone()),
    RuntimeValue::Number(number) => Some(number.to_string()),
    RuntimeValue::Bool(value) => Some(value.to_string()),
    RuntimeValue::Null => Some(String::new()),
    _ => None,
  }
}

fn escape(text: &str) -> String {
  let mut escaped = String::with_capacity(text.len());
  for c in text.chars() {
    match c {
      '&' => escaped.push_str("&amp;"),
      '<' => escaped.push_str("&lt;"),
      '>' => escaped.push_str("&gt;"),
      '"' => escaped.push_str("&quot;"),
      '\'' => escaped.push_str("&apos;"),
      c => escaped.push(c),
    }
  }
  escaped
}

fn require_xml(content_type: &str) -> Result<(), ComponentError> {
  if is_xml(content_type) {
    Ok(())
  } else {
    Err(ComponentError::unsupported_content_type(content_type))
  }
}

/// A slot's content as a document: `base64` octets or text are parsed; a document already decoded,
/// as a contract records one, is taken as it is.
fn slot_document(content_type: &str, value: &SlotValue) -> Result<RuntimeValue, ComponentError> {
  let failed = |message: String| ComponentError::decode_failed(content_type, message);
  match (&value.content, value.encoded.as_deref()) {
    (Value::String(content), Some("base64")) => {
      let bytes = BASE64
        .decode(content)
        .map_err(|err| failed(format!("invalid base64: {err}")))?;
      parse(&bytes).map_err(failed)
    }
    (Value::String(content), None | Some("text") | Some("json")) => parse(content.as_bytes()).map_err(failed),
    (Value::Object(_), None | Some("json")) => Ok(RuntimeValue::from_json(&value.content)),
    (_, Some(other)) if !matches!(other, "base64" | "text" | "json") => {
      Err(failed(format!("unrecognised content encoding '{other}'")))
    }
    _ => Err(failed(
      "an XML slot's content must be its text or an object holding its root element".to_string(),
    )),
  }
}

impl ContentComponent for XmlContent {
  fn handles(&self, content_type: &str) -> bool {
    is_xml(content_type)
  }

  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    require_xml(&req.content_type)?;
    Ok(DecodeResult {
      document: slot_document(&req.content_type, &req.value)?,
      degradations: Vec::new(),
    })
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    require_xml(&req.content_type)?;
    let body =
      write(&req.document).map_err(|message| ComponentError::encode_failed(&req.content_type, message))?;
    Ok(EncodeResult {
      value: SlotValue {
        content: Value::String(BASE64.encode(body.as_bytes())),
        encoded: Some("base64".to_string()),
        content_type: Some(req.content_type),
      },
    })
  }

  /// Nothing to contribute (spec §6.3): an XML document is objects, arrays and strings, which the
  /// generic plan handles.
  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    require_xml(&req.content_type)?;
    Ok(CompileResult {
      fragment: None,
      grammar_version: None,
    })
  }

  /// Not implemented (spec §6.5): an XML body is recognised by its declared type, as a form is.
  fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
    Err(ComponentError::operation_unsupported("content/detect"))
  }
}
