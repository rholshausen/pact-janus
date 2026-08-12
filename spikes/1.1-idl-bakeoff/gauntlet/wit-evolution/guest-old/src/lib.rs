// The "old plugin": compiled against pact:gauntlet@0.1.0, before E1/E4 existed.
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
}

bindings::export!(Component with_types_in bindings);
