//! The transport interface's request/result documents and native-binding trait
//! (component-interfaces spec §5, `schemas/v1/transport.schema.json`): five role-neutral
//! primitives plus a disposition. Field names and structure mirror the schema; `instance` is
//! always engine-assigned (spec §3.4) and carried as a plain `String` rather than a wrapper type.

use super::ComponentError;
use super::parts::Parts;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Start {
  pub instance: String,
  pub kind: String,
  pub role: String,
  pub options: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StartResult {
  /// An open `EndpointDescriptor` (spec §4): host/port for HTTP, broker/topic for messaging,
  /// whatever else a transport needs — never assumed to be a URL.
  pub endpoint: Value,
}

#[derive(Debug, Clone)]
pub struct Stop {
  pub instance: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StopResult {}

#[derive(Debug, Clone)]
pub struct Send {
  pub instance: String,
  pub parts: Parts,
  pub await_reply: bool,
  pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SendResult {
  pub reply: Option<Parts>,
}

#[derive(Debug, Clone)]
pub struct PollInbound {
  pub instance: String,
  pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PollInboundResult {
  pub inbound: Option<Inbound>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Inbound {
  pub event: String,
  pub parts: Parts,
  pub expects_reply: bool,
}

#[derive(Debug, Clone)]
pub struct Reply {
  pub instance: String,
  pub event: String,
  pub parts: Parts,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReplyResult {}

#[derive(Debug, Clone)]
pub struct Dispose {
  pub instance: String,
  pub event: String,
  pub disposition: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisposeResult {}

/// Native binding (spec §9.1): one method per operation, taking that operation's request document
/// and returning `Result<ResultDocument, ComponentError>` — the same documents a byte-pipe binding
/// would carry, passed as in-memory values instead.
pub trait TransportComponent {
  fn start(&self, req: Start) -> Result<StartResult, ComponentError>;
  fn stop(&self, req: Stop) -> Result<StopResult, ComponentError>;
  fn send(&self, req: Send) -> Result<SendResult, ComponentError>;
  fn poll_inbound(&self, req: PollInbound) -> Result<PollInboundResult, ComponentError>;
  fn reply(&self, req: Reply) -> Result<ReplyResult, ComponentError>;
  fn dispose(&self, req: Dispose) -> Result<DisposeResult, ComponentError>;
}
