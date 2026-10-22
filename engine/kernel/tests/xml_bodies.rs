//! ADR 0039: the pact's side and the reply's side of an XML body must be the same document, or a
//! verdict compares two models. The legacy layer reads a pact's XML body (`legacy::xml`) and the
//! built-in XML component decodes a reply's (`engine/component-xml`), each with `sxd-document`, so
//! this holds them to each other over every XML body this repository has.

use pact_janus_kernel::legacy::xml;
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::path::Path;

/// Every string in `value` that looks like an XML document.
fn xml_strings(value: &Value, out: &mut Vec<String>) {
  match value {
    Value::String(text) if text.trim_start().starts_with('<') => out.push(text.clone()),
    Value::Array(items) => items.iter().for_each(|item| xml_strings(item, out)),
    Value::Object(members) => members.values().for_each(|member| xml_strings(member, out)),
    _ => {}
  }
}

fn json_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
  for entry in std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display())) {
    let path = entry.expect("a directory entry").path();
    if path.is_dir() {
      json_files(&path, out);
    } else if path.extension().is_some_and(|extension| extension == "json") {
      out.push(path);
    }
  }
}

#[test]
fn the_legacy_reader_and_the_xml_component_read_every_body_alike() {
  let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
  let mut files = Vec::new();
  json_files(&fixtures, &mut files);
  let mut bodies = Vec::new();
  for file in files {
    let text = std::fs::read_to_string(&file).expect("readable");
    if let Ok(json) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
      xml_strings(&json, &mut bodies);
    }
  }
  assert!(
    bodies.len() > 150,
    "expected the specification cases' XML, found {}",
    bodies.len()
  );

  let mut compared = 0;
  for body in &bodies {
    let legacy = xml::document(body.as_bytes());
    let component = pact_janus_component_xml::parse(body.as_bytes());
    assert_eq!(
      legacy.is_ok(),
      component.is_ok(),
      "{body}: {legacy:?} / {component:?}"
    );
    if let (Ok(legacy), Ok(component)) = (legacy, component) {
      assert_eq!(legacy, component, "{body}");
      compared += 1;
    }
  }
  assert!(compared > 150, "only {compared} bodies parsed");
}
