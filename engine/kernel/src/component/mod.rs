//! The component interfaces (design 2.6, plan task 4.2): what the kernel loads behind it. This
//! module is the **native binding** (spec §9.1) — traits and in-memory request/result documents,
//! implemented by built-in component crates (`engine/component-http`, `engine/component-json`)
//! that this crate deliberately does not depend on, so the kernel itself stays free of HTTP/JSON
//! knowledge (CLAUDE.md's B3, kernel-boundary-review.md).
//!
//! Out of scope here, deliberately: the WASM and subprocess bindings themselves, which need a
//! runtime this crate cannot depend on and which an embedding registers as a [`ComponentLoader`]
//! (plan task 8.1; `engine/component-host` is the WASM one). Of the matcher interface only `apply`
//! is here ([`matcher`], plan task 8.4): it is what executes a fragment's component actions. What
//! *is* here is everything around a load that must not differ by binding:
//! resolution, the handshake checks, requirements ([`loader`]) and routing by media type
//! ([`registry`]). The hook interface ([`hook`]) does, as of plan task 5.3: the hook *system*
//! around it is design 2.7's and lives in [`crate::hooks`]. See
//! `Documentation/specs/component-interfaces/spec.md`.

mod content;
mod error;
mod hook;
mod loader;
mod matcher;
mod parts;
mod registry;
mod transport;

pub use content::{
  Compile, CompileResult, ContentComponent, Decode, DecodeResult, Detect, DetectResult, Encode, EncodeResult,
};
pub use error::ComponentError;
pub use hook::{HookComponent, Invoke};
pub use loader::{
  CORE_FAMILIES, ComponentDeclaration, ComponentLoader, FsGrant, Grants, InTree, Limits, Loaded, Resolved,
  Source, Unavailable, check_requirements, resolve,
};
pub use matcher::{Apply, ApplyResult, MatcherComponent};
pub use parts::{Part, Parts, SlotValue};
pub use registry::{ContentRegistry, media_type_matches};
pub use transport::{
  ContentSlots, Dispose, DisposeResult, Inbound, PollInbound, PollInboundResult, Reply, ReplyResult, Send,
  SendResult, Start, StartResult, Stop, StopResult, TransportComponent,
};
