//! Plan task 4.2: the built-in JSON content component (component-interfaces spec §6) — decode,
//! encode, their round trip (spec §6.2, load-bearing not a nicety), detection (kernel-boundary-
//! review.md finding 1's replacement for the kernel's old byte-sniff), and `compile`'s legitimate
//! "nothing to contribute" answer (spec §6.3).

use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::{Compile, ContentComponent, Decode, Detect, Encode, SlotValue};
use pact_janus_kernel::plan::{ContentDetector, RuntimeValue};
use serde_json::json;

fn json_slot(text: &str) -> SlotValue {
  SlotValue {
    content: json!(text),
    encoded: Some("text".to_string()),
    content_type: Some("application/json".to_string()),
  }
}

#[test]
fn decode_parses_a_json_document() {
  let component = JsonContent::new();
  let result = component
    .decode(Decode {
      content_type: "application/json".to_string(),
      value: json_slot(r#"{"id":42,"tags":["a","b"]}"#),
      options: None,
    })
    .expect("well-formed JSON decodes");
  assert_eq!(
    result.document,
    RuntimeValue::from_json(&json!({ "id": 42, "tags": ["a", "b"] }))
  );
  assert!(result.degradations.is_empty());
}

#[test]
fn decode_reports_malformed_json_as_decode_failed() {
  let component = JsonContent::new();
  let err = component
    .decode(Decode {
      content_type: "application/json".to_string(),
      value: json_slot("{not json"),
      options: None,
    })
    .expect_err("malformed JSON must not decode");
  assert_eq!(err.code, "decode-failed");
  assert_eq!(err.category, "component");
}

#[test]
fn decode_rejects_a_content_type_it_does_not_handle() {
  let component = JsonContent::new();
  let err = component
    .decode(Decode {
      content_type: "application/xml".to_string(),
      value: json_slot("<a/>"),
      options: None,
    })
    .expect_err("this component only speaks JSON");
  assert_eq!(err.code, "unsupported-content-type");
}

#[test]
fn decode_then_encode_round_trips() {
  let component = JsonContent::new();
  let document = RuntimeValue::from_json(&json!({ "id": 42, "discount": null, "tags": [1, 2] }));
  let encoded = component
    .encode(Encode {
      content_type: "application/json".to_string(),
      document: document.clone(),
      options: None,
    })
    .expect("a RuntimeValue always encodes to JSON");
  let decoded = component
    .decode(Decode {
      content_type: "application/json".to_string(),
      value: encoded.value,
      options: None,
    })
    .expect("what this component just encoded, it can decode");
  assert_eq!(
    decoded.document, document,
    "decode(encode(d)) must equal d (spec §6.2)"
  );
}

#[test]
fn encode_produces_base64_octets_with_the_declared_content_type() {
  let component = JsonContent::new();
  let encoded = component
    .encode(Encode {
      content_type: "application/json".to_string(),
      document: RuntimeValue::from_json(&json!({"a": 1})),
      options: None,
    })
    .unwrap();
  assert_eq!(encoded.value.encoded.as_deref(), Some("base64"));
  assert_eq!(encoded.value.content_type.as_deref(), Some("application/json"));
}

#[test]
fn compile_returns_no_fragment() {
  let component = JsonContent::new();
  let result = component
    .compile(Compile {
      content_type: "application/json".to_string(),
      shape: json!({ "shape": "object", "members": {} }),
      path: "$.response.body".to_string(),
    })
    .unwrap();
  assert!(
    result.fragment.is_none(),
    "the kernel compiles the slot generically (spec §6.3)"
  );
}

#[test]
fn detect_recognises_json_octets() {
  let component = JsonContent::new();
  let result = ContentComponent::detect(
    &component,
    Detect {
      value: SlotValue {
        content: json!(r#"{"a":1}"#),
        encoded: Some("text".to_string()),
        content_type: None,
      },
      hint: None,
    },
  )
  .unwrap();
  assert_eq!(result.media_type.as_deref(), Some("application/json"));
}

#[test]
fn detect_does_not_claim_non_json_octets() {
  let component = JsonContent::new();
  let result = ContentComponent::detect(
    &component,
    Detect {
      value: SlotValue {
        content: json!("<?xml version=\"1.0\"?><a/>"),
        encoded: Some("text".to_string()),
        content_type: None,
      },
      hint: None,
    },
  )
  .unwrap();
  assert!(result.media_type.is_none());
}

#[test]
fn content_detector_adapter_matches_the_interpreters_expectations() {
  // The same path plan::interpret's match:content-type calls through execute_with_content.
  let component = JsonContent::new();
  let detector: &dyn ContentDetector = &component;
  assert_eq!(
    detector.detect(&RuntimeValue::String(r#"{"a":1}"#.to_string())),
    Some("application/json".to_string())
  );
  assert_eq!(
    detector.detect(&RuntimeValue::String("plain text".to_string())),
    None
  );
  assert_eq!(detector.detect(&RuntimeValue::Bool(true)), None);
}
