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
  let content_type = declared.or(slot.content_type.as_deref());
  let claimed = content_type
    .zip(content)
    .filter(|(content_type, content)| content.handles(content_type));
  match claimed {
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
pub(crate) fn mismatch_json(mismatch: &Mismatch) -> Value {
  serde_json::json!({
    "path": mismatch.path,
    "message": mismatch.message,
    "action": mismatch.action,
  })
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
  fn a_plain_value_is_taken_as_it_stands() {
    assert_eq!(
      decode_slot(&plain_slot(&json!(200)), None, Some(&Failing)),
      RuntimeValue::from_json(&json!(200))
    );
  }
}
