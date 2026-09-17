//! Wire-form helpers shared by the two sides of the engine that put parts on a transport and read
//! them back: the consumer session's live exchange ([`super::exchange`], plan task 4.5) and the
//! verification run ([`super::verification`], plan task 5.1).
//!
//! They belong together because the two sides are the same conversion in opposite directions —
//! a mock decodes an inbound request and encodes a reply, a verifier encodes an outbound request
//! and decodes the reply — and the *one* thing that must not drift between them is what a slot's
//! document value is. A recorded request that replayed differently from how it was matched would
//! make every variant's evidence meaningless.

use crate::component::{ContentComponent, Decode, Encode, Parts, SlotValue};
use crate::plan::{CapturedValues, Executed, ExecutedKind, Mismatch, RuntimeValue};
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
/// roots every slot at (shape spec §6.2).
pub(crate) fn parts_resolver(parts: &Parts, content: Option<&dyn ContentComponent>) -> CapturedValues {
  let mut resolver = CapturedValues::new();
  for (part_name, slots) in parts {
    for (slot_name, slot_value) in slots {
      let path = format!("$.{part_name}.{slot_name}");
      resolver = resolver.capture(path, decode_slot(slot_value, content));
    }
  }
  resolver
}

/// A wire-form slot's document value: a content component decodes anything it tagged with a
/// content type (component-interfaces spec §6); everything else — method, path, headers, an
/// untyped body — is already the document model's own value (contract-file spec §5.3's `json`
/// default), so it is taken as-is.
pub(crate) fn decode_slot(slot: &SlotValue, content: Option<&dyn ContentComponent>) -> RuntimeValue {
  match (&slot.content_type, content) {
    (Some(content_type), Some(content)) => match content.decode(Decode {
      content_type: content_type.clone(),
      value: slot.clone(),
      options: None,
    }) {
      Ok(result) => result.document,
      // Undecodable is "not there", not a crash: the plan discovers it via `check:exists`/a
      // failed match, same as any other absent value (plan-grammar spec §2.4).
      Err(_) => RuntimeValue::Absent,
    },
    _ => RuntimeValue::from_json(&slot.content),
  }
}

/// A generated document value, wired for reply: a scalar rides as the transport's plain wire form
/// (component-http reads `status` this way, no content component involved); anything structured
/// has no wire form except through a content component's encoding, so it gets one.
///
/// Which content type a slot actually wants is really a per-slot property of the interaction's
/// declared shape (design 2.6) — this loop doesn't have that wiring (the same "single slot, not a
/// registry" gap [`crate::plan::resolve`] already documents), so it infers structured-vs-scalar
/// from the generated value instead. Correct for the RFC order interaction's JSON body; whoever
/// adds a second content type resolves this properly.
pub(crate) fn encode_slot(value: &Value, content: Option<&dyn ContentComponent>) -> SlotValue {
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
