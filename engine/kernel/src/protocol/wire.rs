//! Wire-form helpers shared by the two sides of the engine that put parts on a transport and read
//! them back: the consumer session's live exchange ([`super::exchange`], plan task 4.5) and the
//! verification run ([`super::verification`], plan task 5.1).
//!
//! They belong together because the two sides are the same conversion in opposite directions —
//! a mock decodes an inbound request and encodes a reply, a verifier encodes an outbound request
//! and decodes the reply — and the *one* thing that must not drift between them is what a slot's
//! document value is. A recorded request that replayed differently from how it was matched would
//! make every variant's evidence meaningless.

use crate::common::{ContentTypes, declared_type};
use crate::component::{ContentComponent, Decode, Encode, Parts, SlotValue};
use crate::plan::{CapturedValues, Executed, ExecutedKind, Mismatch, RuntimeValue};
use crate::variant::generate::Generated;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;

/// The plan-executed tree's named part subtree (`"request"`/`"response"`) — the root
/// [`execute`](crate::plan::execute) produces is always a container of part containers
/// (`plan::compile`'s own shape), so this is a one-level lookup, not a general tree search.
pub(crate) fn find_container<'a>(executed: &'a Executed, label: &str) -> Option<&'a Executed> {
  let ExecutedKind::Container { children, .. } = &executed.kind else {
    return None;
  };
  children
    .iter()
    .find(|child| matches!(&child.kind, ExecutedKind::Container { label: Some(l), .. } if l == label))
}

/// A resolver over an inbound arrival's wire-form parts (component-interfaces spec §4), decoded to
/// the document model a plan resolves against — the same `$.<part>.<slot>` paths `plan::compile`
/// roots every slot at (shape spec §6.2). `declared` is the interaction's own `content-types`
/// (contract-file spec §5.5): a declared slot is decoded as the interaction said, not as the peer
/// labelled it.
pub(crate) fn parts_resolver(
  parts: &Parts,
  declared: &ContentTypes,
  content: Option<&dyn ContentComponent>,
) -> CapturedValues {
  let mut resolver = CapturedValues::new();
  for (part_name, slots) in parts {
    for (slot_name, slot_value) in slots {
      let path = format!("$.{part_name}.{slot_name}");
      let declared = declared_type(Some(declared), part_name, slot_name);
      resolver = resolver.capture(path, decode_slot(slot_value, declared, content));
    }
  }
  resolver
}

/// A wire-form slot's document value: a content component decodes anything with a content type —
/// the one the interaction declared for the slot, else the one it arrived labelled with
/// (component-interfaces spec §6, contract-file spec §5.5). Octets no component claims, with a type
/// or without one, are the document model's bytes (ADR 0036 decision 2). Everything else — method,
/// path, headers, a body carried as its JSON value — is already the document model's own value
/// (contract-file spec §5.3's `json` default), so it is taken as-is.
pub(crate) fn decode_slot(
  slot: &SlotValue,
  declared: Option<&str>,
  content: Option<&dyn ContentComponent>,
) -> RuntimeValue {
  match claimant(slot, declared, content) {
    Some((content_type, content)) => match content.decode(Decode {
      content_type: content_type.to_string(),
      value: slot.clone(),
      options: None,
    }) {
      Ok(result) => result.document,
      // Undecodable is "not there", not a crash: the plan discovers it via `check:exists`/a
      // failed match, same as any other absent value (plan-grammar spec §2.4).
      Err(error) => {
        tracing::debug!(
          content_type,
          ?error,
          "slot could not be decoded; matching it as absent"
        );
        RuntimeValue::Absent
      }
    },
    None => match (slot.encoded.as_deref(), slot.content.as_str()) {
      (Some("base64"), Some(text)) => BASE64
        .decode(text)
        .map(RuntimeValue::Bytes)
        .unwrap_or(RuntimeValue::Absent),
      _ => RuntimeValue::from_json(&slot.content),
    },
  }
}

/// The content type a slot is decoded as and the component that claims it, if one does: the type
/// declared for the slot, else the one it is labelled with.
/// A recorded slot's document value (ADR 0086 decision 3), decoded as a reply's is by [`decode_slot`],
/// except that octets a component claims and cannot decode are kept as octets rather than read as
/// absent: a recorder records what the provider sent. The second value says why, when it did.
pub(crate) fn recorded_slot(
  slot: &SlotValue,
  content: Option<&dyn ContentComponent>,
) -> (RuntimeValue, Option<String>) {
  let Some((content_type, component)) = claimant(slot, None, content) else {
    return (decode_slot(slot, None, None), None);
  };
  match component.decode(Decode {
    content_type: content_type.to_string(),
    value: slot.clone(),
    options: None,
  }) {
    Ok(result) => (result.document, None),
    Err(error) => (
      decode_slot(slot, None, None),
      Some(format!(
        "the value labelled '{content_type}' could not be decoded, so its octets were recorded: {}",
        error.message
      )),
    ),
  }
}

/// The type a produced message has (ADR 0067 decision 3), as today's verifier takes it: its body's
/// label, else its metadata's own `contentType`, else what the content registry detects its octets
/// as. A verification checks the contract's `contentType` against it, and a recording reads an
/// unlabelled body by it (ADR 0090 decision 3), so both read one message the same way.
pub(crate) fn produced_message_type(
  message: &crate::component::Part,
  content: Option<&dyn ContentComponent>,
) -> Option<String> {
  let body = message.get("body");
  let labelled = body.and_then(|body| body.content_type.clone());
  let own = || {
    message
      .get("metadata")
      .and_then(|metadata| metadata.content.get("contentType"))
      .and_then(Value::as_str)
      .filter(|content_type| !content_type.is_empty())
      .map(str::to_string)
  };
  let detected = || {
    let (body, content) = (body?, content?);
    content
      .detect(crate::component::Detect {
        value: body.clone(),
        hint: None,
      })
      .ok()?
      .media_type
  };
  labelled.or_else(own).or_else(detected)
}

fn claimant<'a, 'c>(
  slot: &'a SlotValue,
  declared: Option<&'a str>,
  content: Option<&'c dyn ContentComponent>,
) -> Option<(&'a str, &'c dyn ContentComponent)> {
  declared
    .or(slot.content_type.as_deref())
    .zip(content)
    .filter(|(content_type, content)| content.handles(content_type))
}

/// A recorded example's slot as a mock serves it (ADR 0066 decision 4, S2.15): decoded as a reply's
/// is ([`decode_slot`]), except that octets no component claims are served as those octets, whether
/// they were recorded as `base64` or as UTF-8 `text`. `None` is a slot that could not be decoded.
pub(crate) fn served_slot(
  slot: &SlotValue,
  declared: Option<&str>,
  content: Option<&dyn ContentComponent>,
) -> Option<Generated> {
  let unclaimed_text = claimant(slot, declared, content).is_none() && slot.encoded.as_deref() == Some("text");
  if unclaimed_text && let Some(text) = slot.content.as_str() {
    return Some(Generated::Octets(text.as_bytes().to_vec()));
  }
  match decode_slot(slot, declared, content) {
    RuntimeValue::Absent => None,
    RuntimeValue::Bytes(bytes) => Some(Generated::Octets(bytes)),
    document => Some(Generated::Document(document.to_json())),
  }
}

/// A generated document value, wired for a transport. A slot the interaction declared a content type
/// for (contract-file spec §5.5) is encoded through the component for that type, whatever the value
/// is. An undeclared one keeps the default the declaration exists to override: a scalar rides as the
/// transport's plain wire form (component-http reads `status` this way, no content component
/// involved), and anything structured is encoded as `application/json`.
pub(crate) fn encode_slot(
  value: &Value,
  declared: Option<&str>,
  content: Option<&dyn ContentComponent>,
) -> SlotValue {
  if let (Some(content_type), Some(content)) = (declared, content) {
    return match content.encode(Encode {
      content_type: content_type.to_string(),
      document: RuntimeValue::from_json(value),
      options: None,
    }) {
      Ok(result) => result.value,
      Err(error) => {
        tracing::warn!(
          content_type,
          ?error,
          "a declared content slot could not be encoded; sending its plain value"
        );
        plain_slot(value)
      }
    };
  }
  match (value, content) {
    (Value::Object(_) | Value::Array(_), Some(content)) => match content.encode(Encode {
      content_type: "application/json".to_string(),
      document: RuntimeValue::from_json(value),
      options: None,
    }) {
      Ok(result) => result.value,
      Err(_) => plain_slot(value),
    },
    _ => plain_slot(value),
  }
}

/// Octets wired as octets (S2.15): `base64`, with the type declared for the slot, if any. No content
/// component encodes them, since they are already what a transport carries.
pub(crate) fn octets_slot(bytes: &[u8], content_type: Option<&str>) -> SlotValue {
  SlotValue {
    content: Value::String(BASE64.encode(bytes)),
    encoded: Some("base64".to_string()),
    content_type: content_type.map(str::to_string),
  }
}

pub(crate) fn plain_slot(value: &Value) -> SlotValue {
  SlotValue {
    content: value.clone(),
    encoded: None,
    content_type: None,
  }
}

/// A mismatch as the protocol carries it (spec §9.6's event payloads, and the mock's own
/// mismatch reply): path and message, never the executed node — `explain --executed` is how a
/// caller asks for the whole tree (plan task 3.6).
pub(crate) fn mismatch_json(mismatch: &Mismatch, segments: bool) -> Value {
  let mut json = serde_json::json!({
    "path": mismatch.path,
    "message": mismatch.message,
    "action": mismatch.action,
  });
  // The values it quotes, marked, for a host that asked (spec §2.7, ADR 0054). A message that quotes
  // none is one plain segment.
  if segments {
    let marked = mismatch
      .marks
      .as_deref()
      .cloned()
      .unwrap_or_else(|| crate::display::Marked::plain(mismatch.message.clone()));
    super::display::put(&mut json, "message", &marked, true);
  }
  json
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::component::{
    Compile, CompileResult, ComponentError, DecodeResult, Detect, DetectResult, EncodeResult,
  };
  use pretty_assertions::assert_eq;
  use serde_json::json;

  /// Claims `text/plain` and fails every decode.
  struct Failing;

  impl ContentComponent for Failing {
    fn handles(&self, content_type: &str) -> bool {
      content_type == "text/plain"
    }
    fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
      Err(ComponentError::decode_failed(&req.content_type, "never"))
    }
    fn encode(&self, _req: Encode) -> Result<EncodeResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/encode"))
    }
    fn compile(&self, _req: Compile) -> Result<CompileResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/compile"))
    }
    fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/detect"))
    }
  }

  fn octets(content_type: Option<&str>) -> SlotValue {
    SlotValue {
      content: json!("AAEC"),
      encoded: Some("base64".to_string()),
      content_type: content_type.map(str::to_string),
    }
  }

  #[test]
  fn octets_no_component_claims_are_bytes() {
    for content_type in [Some("image/png"), None] {
      assert_eq!(
        decode_slot(&octets(content_type), None, Some(&Failing)),
        RuntimeValue::Bytes(vec![0, 1, 2]),
        "{content_type:?}"
      );
    }
    assert_eq!(
      decode_slot(&octets(Some("image/png")), None, None),
      RuntimeValue::Bytes(vec![0, 1, 2]),
      "with no content component at all"
    );
  }

  #[test]
  fn octets_a_component_claims_and_cannot_decode_are_absent() {
    assert_eq!(
      decode_slot(&octets(Some("text/plain")), None, Some(&Failing)),
      RuntimeValue::Absent
    );
  }

  #[test]
  fn a_declared_type_decides_who_claims_the_octets() {
    assert_eq!(
      decode_slot(&octets(Some("image/png")), Some("text/plain"), Some(&Failing)),
      RuntimeValue::Absent
    );
  }

  #[test]
  fn an_example_no_component_claims_is_served_as_its_octets() {
    let text = SlotValue {
      content: json!("\u{0}\u{1}\u{2}"),
      encoded: Some("text".to_string()),
      content_type: Some("application/octet-stream".to_string()),
    };
    assert_eq!(
      served_slot(&text, None, Some(&Failing)),
      Some(Generated::Octets(vec![0, 1, 2]))
    );
    assert_eq!(
      served_slot(&octets(None), None, Some(&Failing)),
      Some(Generated::Octets(vec![0, 1, 2]))
    );
    assert_eq!(
      served_slot(&plain_slot(&json!({ "id": 1 })), None, Some(&Failing)),
      Some(Generated::Document(json!({ "id": 1 })))
    );
  }

  #[test]
  fn an_example_a_component_claims_is_its_document_or_nothing() {
    let text = SlotValue {
      content: json!("hello"),
      encoded: Some("text".to_string()),
      content_type: Some("text/plain".to_string()),
    };
    assert_eq!(
      served_slot(&text, None, Some(&Failing)),
      None,
      "it could not be decoded"
    );
  }

  #[test]
  fn a_plain_value_is_taken_as_it_stands() {
    assert_eq!(
      decode_slot(&plain_slot(&json!(200)), None, Some(&Failing)),
      RuntimeValue::from_json(&json!(200))
    );
  }
}
