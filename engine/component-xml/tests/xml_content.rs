//! The built-in XML content component (ADR 0039): what an XML body decodes to, what a document
//! encodes to, and which types it claims.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_xml::{XmlContent, parse, write};
use pact_janus_kernel::component::{ContentComponent, Decode, Encode, SlotValue};
use pact_janus_kernel::plan::RuntimeValue;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::{Value, json};

fn document(value: Value) -> RuntimeValue {
  RuntimeValue::from_json(&value)
}

#[test]
fn an_element_is_its_attributes_text_and_children_keyed_by_name() {
  let body = r#"<?xml version="1.0" encoding="UTF-8"?>
    <alligator name="Mary" age="3">
      <favouriteColours>
        <favouriteColour>red</favouriteColour>
        <favouriteColour>blue</favouriteColour>
      </favouriteColours>
      <note>teeth</note>
    </alligator>"#;
  assert_eq!(
    parse(body.as_bytes()),
    Ok(document(json!({
      "alligator": {
        "@age": "3",
        "@name": "Mary",
        "favouriteColours": [{
          "favouriteColour": [ { "#text": "red" }, { "#text": "blue" } ]
        }],
        "note": [ { "#text": "teeth" } ]
      }
    })))
  );
}

#[test]
fn children_keep_the_order_they_arrived_in_and_attributes_are_in_name_order() {
  let parsed = parse(br#"<a z="1" y="2"><c/><b/><c/></a>"#).expect("parses");
  let RuntimeValue::Object(root) = &parsed else {
    panic!("an object")
  };
  let RuntimeValue::Object(a) = &root["a"] else {
    panic!("an element")
  };
  assert_eq!(a.keys().collect::<Vec<_>>(), vec!["@y", "@z", "c", "b"]);
}

#[test]
fn text_is_each_text_node_trimmed_then_joined_and_absent_when_empty() {
  assert_eq!(
    parse(b"<a> one <b/> two <![CDATA[ <three> ]]><!-- not text --></a>"),
    Ok(document(
      json!({ "a": { "#text": "onetwo<three>", "b": [ {} ] } })
    ))
  );
  assert_eq!(parse(b"<a>  \n  </a>"), Ok(document(json!({ "a": {} }))));
}

#[test]
fn a_namespace_is_a_member_and_a_prefix_is_not_kept() {
  let body = r#"<a:alligator xmlns:a="urn:alligators" xmlns:n="urn:names" n:name="Mary" id="1">
      <a:favouriteNumbers><favouriteNumber xmlns="urn:favourite:numbers">1</favouriteNumber></a:favouriteNumbers>
    </a:alligator>"#;
  assert_eq!(
    parse(body.as_bytes()),
    Ok(document(json!({
      "alligator": {
        "#namespace": "urn:alligators",
        "@id": "1",
        "@{urn:names}name": "Mary",
        "favouriteNumbers": [{
          "#namespace": "urn:alligators",
          "favouriteNumber": [ { "#namespace": "urn:favourite:numbers", "#text": "1" } ]
        }]
      }
    })))
  );
}

#[test]
fn a_default_namespace_applies_to_elements_and_not_to_attributes() {
  assert_eq!(
    parse(br#"<alligator xmlns="urn:alligators" name="Mary"/>"#),
    Ok(document(
      json!({ "alligator": { "#namespace": "urn:alligators", "@name": "Mary" } })
    ))
  );
}

#[rstest]
#[case::not_xml(b"{\"a\": 1}".as_slice())]
#[case::unclosed(b"<a><b></a>".as_slice())]
#[case::empty(b"".as_slice())]
#[case::not_utf8(b"<a>\xff</a>".as_slice())]
fn a_body_that_is_not_well_formed_xml_does_not_parse(#[case] body: &[u8]) {
  assert!(parse(body).is_err(), "{:?} parsed", String::from_utf8_lossy(body));
}

#[test]
fn a_document_writes_as_xml_with_text_before_children() {
  let written = write(&document(json!({
    "alligator": {
      "@name": "Mary & Fred",
      "#text": "a < b",
      "favouriteColour": [ { "#text": "red" }, { "#text": "blue" } ],
      "empty": [ {} ]
    }
  })));
  assert_eq!(
    written.as_deref(),
    Ok(
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?><alligator name=\"Mary &amp; Fred\">a &lt; b\
       <favouriteColour>red</favouriteColour><favouriteColour>blue</favouriteColour><empty/></alligator>"
    )
  );
}

#[test]
fn namespaces_are_declared_where_they_change() {
  let written = write(&document(json!({
    "alligator": {
      "#namespace": "urn:alligators",
      "@{urn:names}name": "Mary",
      "@{http://www.w3.org/XML/1998/namespace}lang": "en",
      "inner": [ { "#namespace": "urn:alligators" } ],
      "plain": [ {} ],
      "other": [ { "#namespace": "urn:other" } ]
    }
  })));
  assert_eq!(
    written.as_deref(),
    Ok(
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?><alligator xmlns=\"urn:alligators\" xmlns:ns0=\"urn:names\" \
       ns0:name=\"Mary\" xml:lang=\"en\"><inner/><plain xmlns=\"\"/><other xmlns=\"urn:other\"/></alligator>"
    )
  );
}

#[rstest]
#[case::simple(r#"<a x="1"><b>one</b><b>two</b><c y="z"/></a>"#)]
#[case::namespaced(r#"<p:a xmlns:p="urn:p" xmlns:q="urn:q" q:x="1"><b xmlns="urn:b">t</b><p:c/></p:a>"#)]
#[case::escaped(r#"<a x="&quot;&amp;&lt;">&lt;&amp;&gt;</a>"#)]
fn what_is_written_parses_to_the_same_document(#[case] body: &str) {
  let parsed = parse(body.as_bytes()).expect("parses");
  let written = write(&parsed).expect("writes");
  assert_eq!(parse(written.as_bytes()), Ok(parsed), "{written}");
}

#[test]
fn numbers_and_booleans_are_written_as_their_text() {
  assert_eq!(
    write(&document(json!({ "a": { "@id": 10, "#text": true } }))).as_deref(),
    Ok("<?xml version=\"1.0\" encoding=\"UTF-8\"?><a id=\"10\">true</a>")
  );
}

#[rstest]
#[case::not_an_object(json!("text"))]
#[case::two_roots(json!({ "a": {}, "b": {} }))]
#[case::no_root(json!({}))]
#[case::an_element_that_is_not_an_object(json!({ "a": "text" }))]
#[case::an_attribute_that_is_not_text(json!({ "a": { "@x": { "y": 1 } } }))]
#[case::an_unknown_hash_member(json!({ "a": { "#comment": "no" } }))]
#[case::a_name_that_is_not_a_name(json!({ "a b": {} }))]
fn a_document_that_is_not_xml_shaped_does_not_write(#[case] value: Value) {
  assert!(write(&document(value.clone())).is_err(), "{value} wrote");
}

#[rstest]
#[case("application/xml", true)]
#[case("text/xml; charset=UTF-8", true)]
#[case("application/atom+xml", true)]
#[case("Application/XML", true)]
#[case("image/svg+xml", false)]
#[case("application/json", false)]
#[case("text/plain", false)]
fn it_claims_the_types_todays_verifier_counts_as_xml(#[case] content_type: &str, #[case] claimed: bool) {
  assert_eq!(XmlContent::new().handles(content_type), claimed);
}

#[test]
fn it_decodes_a_slot_and_encodes_a_document_back() {
  let xml = XmlContent::new();
  let body = "<a><b>1</b></a>";
  let decoded = xml
    .decode(Decode {
      content_type: "application/xml".to_string(),
      value: SlotValue {
        content: Value::String(BASE64.encode(body)),
        encoded: Some("base64".to_string()),
        content_type: Some("application/xml".to_string()),
      },
      options: None,
    })
    .expect("decodes");
  assert_eq!(
    decoded.document,
    document(json!({ "a": { "b": [ { "#text": "1" } ] } }))
  );

  let encoded = xml
    .encode(Encode {
      content_type: "text/xml".to_string(),
      document: decoded.document,
      options: None,
    })
    .expect("encodes");
  assert_eq!(encoded.value.content_type.as_deref(), Some("text/xml"));
  let octets = BASE64
    .decode(encoded.value.content.as_str().expect("base64 text"))
    .expect("base64");
  assert_eq!(
    String::from_utf8(octets).expect("UTF-8"),
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><a><b>1</b></a>"
  );
}

#[test]
fn a_slot_that_does_not_parse_fails_to_decode() {
  let failed = XmlContent::new().decode(Decode {
    content_type: "application/xml".to_string(),
    value: SlotValue {
      content: Value::String("<a>".to_string()),
      encoded: None,
      content_type: None,
    },
    options: None,
  });
  assert!(failed.is_err());
}
