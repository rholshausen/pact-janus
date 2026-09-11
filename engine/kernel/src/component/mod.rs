//! The component interfaces (design 2.6, plan task 4.2): what the kernel loads behind it. This
//! module is the **native binding** (spec §9.1) — traits and in-memory request/result documents,
//! implemented by built-in component crates (`engine/component-http`, `engine/component-json`)
//! that this crate deliberately does not depend on, so the kernel itself stays free of HTTP/JSON
//! knowledge (CLAUDE.md's B3, kernel-boundary-review.md).
//!
//! Out of scope here, deliberately: the WASM and subprocess bindings, the handshake
//! (`component/hello`) and its `contributes` vocabulary, and the matcher/hook interfaces — none of
//! them has a caller yet. See `Documentation/specs/component-interfaces/spec.md` and this task's
//! plan-file non-goals.

mod content;
mod error;
mod parts;
mod transport;

pub use content::{
  Compile, CompileResult, ContentComponent, Decode, DecodeResult, Detect, DetectResult, Encode, EncodeResult,
};
pub use error::ComponentError;
pub use parts::{Part, Parts, SlotValue};
pub use transport::{
  Dispose, DisposeResult, Inbound, PollInbound, PollInboundResult, Reply, ReplyResult, Send, SendResult,
  Start, StartResult, Stop, StopResult, TransportComponent,
};
