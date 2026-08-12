// The "new plugin": compiled against the grown interface, and it actually uses the
// new cases (emits hook-invoked, reports component-unavailable).
#[allow(warnings)]
mod bindings;

use bindings::exports::pact::gauntlet::events::{ErrorCode, Guest, VerifyEvent};

struct Component;

impl Guest for Component {
  fn next_event(seq: u32) -> Option<VerifyEvent> {
    match seq {
      0 => Some(VerifyEvent::Started("run".into())),
      1 => Some(VerifyEvent::HookInvoked("before-request".into())),
      2 => Some(VerifyEvent::InteractionResult("ok".into())),
      3 => Some(VerifyEvent::Finished("done".into())),
      _ => None,
    }
  }

  fn last_error() -> ErrorCode {
    ErrorCode::ComponentUnavailable
  }
}

bindings::export!(Component with_types_in bindings);
