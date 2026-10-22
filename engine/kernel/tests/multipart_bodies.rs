//! ADR 0040: the pact's side and the reply's side of a multipart body must be the same document, or a
//! verdict compares two models. The legacy layer reads a pact's multipart body
//! (`legacy::pact::read_multipart`) and the built-in multipart component decodes a reply's
//! (`engine/component-multipart`), each with `multer`, so this holds them to each other over every
//! multipart body this repository has, and a few that reach every part type.

use pact_janus_kernel::legacy::{multipart, pact as legacy_pact};
use pretty_assertions::assert_eq;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Every (body, content type) in `value` whose `headers` name a multipart type.
fn multipart_bodies(value: &Value, out: &mut Vec<(String, String)>) {
  match value {
    Value::Object(members) => {
      let content_type = members
        .get("headers")
        .and_then(Value::as_object)
        .and_then(|headers| {
          headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .and_then(|(_, value)| match value {
              Value::Array(values) => values.first().and_then(Value::as_str),
              value => value.as_str(),
            })
        });
      let body = match members.get("body") {
        Some(Value::String(body)) => Some(body.as_str()),
        Some(Value::Object(body)) => body.get("text").and_then(Value::as_str),
        _ => None,
      };
      if let (Some(content_type), Some(body)) = (content_type, body)
        && content_type.trim().to_ascii_lowercase().starts_with("multipart/")
      {
        out.push((body.to_string(), content_type.to_string()));
      }
      members.values().for_each(|member| multipart_bodies(member, out));
    }
    Value::Array(items) => items.iter().for_each(|item| multipart_bodies(item, out)),
    _ => {}
  }
}

fn json_files(dir: &Path, out: &mut Vec<PathBuf>) {
  for entry in std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display())) {
    let path = entry.expect("a directory entry").path();
    if path.is_dir() {
      json_files(&path, out);
    } else if path.extension().is_some_and(|extension| extension == "json") {
      out.push(path);
    }
  }
}

/// Bodies that reach every way a part is read: a field, JSON, XML, a form, text, octets, base64, a part
/// with no type that is not UTF-8, and a part that is itself multipart.
const EVERY_KIND: &str = "--k\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nMary\r\n\
--k\r\nContent-Disposition: form-data; name=\"order\"; filename=\"o.json\"\r\nContent-Type: application/json\r\n\r\n{\"id\": 1, \"items\": [\"a\"]}\r\n\
--k\r\nContent-Disposition: form-data; name=\"note\"\r\nContent-Type: text/xml\r\n\r\n<note a=\"1\">hi<b/></note>\r\n\
--k\r\nContent-Disposition: form-data; name=\"q\"\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\na=1&a=2&b=x+y\r\n\
--k\r\nContent-Disposition: form-data; name=\"csv\"; filename=\"a.csv\"\r\nContent-Type: text/csv\r\n\r\na,b\r\n\
--k\r\nContent-Disposition: form-data; name=\"logo\"; filename=\"l.png\"\r\nContent-Type: image/png\r\nContent-Transfer-Encoding: base64\r\n\r\niVBORw0KGgo=\r\n\
--k\r\nContent-Disposition: form-data; name=\"order\"; filename=\"p.json\"\r\nContent-Type: application/json\r\n\r\n[]\r\n\
--k\r\nX-Note: n\r\n\r\nunnamed\r\n\
--k\r\nContent-Type: multipart/mixed; boundary=in\r\n\r\n--in--\r\n--k--";

#[test]
fn the_legacy_reader_and_the_multipart_component_read_every_body_alike() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  let mut files = Vec::new();
  json_files(&root.join("engine/kernel/tests/fixtures"), &mut files);
  json_files(&root.join("corpora"), &mut files);
  let mut bodies = vec![(
    EVERY_KIND.to_string(),
    "multipart/form-data; boundary=k".to_string(),
  )];
  for file in files {
    let text = std::fs::read_to_string(&file).expect("readable");
    if let Ok(json) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
      multipart_bodies(&json, &mut bodies);
    }
  }
  assert!(
    bodies.len() >= 5,
    "expected the compatibility suite's multipart bodies and the corpora's, found {}",
    bodies.len()
  );

  for (body, content_type) in &bodies {
    let legacy = legacy_pact::read_multipart(body.as_bytes(), content_type);
    let component = pact_janus_component_multipart::parse(body.as_bytes(), content_type);
    match (legacy, component) {
      (Ok(parts), Ok(component)) => assert_eq!(multipart::document(&parts), component, "{body}"),
      (legacy, component) => panic!("{body}: {legacy:?} / {component:?}"),
    }
  }
}
