// The "new plugin" for E3: compiled against the grown-op interface, exports explain.
// Event/error behaviour matches the old plugin (types are unchanged in E3).
#[allow(warnings)]
mod bindings;

use bindings::exports::pact::gauntlet::events::{ErrorCode, Guest, VerifyEvent};

struct Component;

impl Guest for Component {
  fn next_event(seq: u32) -> Option<VerifyEvent> {
    match seq {
      0 => Some(VerifyEvent::Started("run".into())),
      1 => Some(VerifyEvent::InteractionResult("ok".into())),
      2 => Some(VerifyEvent::Finished("done".into())),
      _ => None,
    }
  }

  fn last_error() -> ErrorCode {
    ErrorCode::Internal
  }

  fn explain(spec: String) -> String {
    format!("plan for: {spec}")
  }
}

bindings::export!(Component with_types_in bindings);
