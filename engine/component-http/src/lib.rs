//! The built-in HTTP transport component (component-interfaces spec §5): the native binding for a
//! real mock HTTP server in the **serve** role (plan task 4.2) and a real HTTP client in the
//! **drive** role (plan task 5.1), in-tree but through the same interface a third-party transport
//! would use (ADR 0012). It is a matcher too ([`status`], ADR 0057): knowing what a status class is
//! and speaking HTTP are the same knowledge (spec §2.1).
//!
//! One component, two roles, five role-neutral primitives (spec §5.1): a served instance answers
//! `poll-inbound`/`reply`/`dispose`, a driven one answers `send`, and each answers
//! `operation-unsupported` for the other's — spec §5.2's rule that "no reply expected" and "reply
//! expected but absent" are different observations applies to whole operations too, so a driven
//! instance asked to poll says so by name rather than returning "nothing arrived" forever.
//!
//! The two halves are rewritten separately (staged plan §3). The drive half ([`drive`], S1.5) sends
//! through the engine's one HTTP client, which is async behind an executor seam and starts no
//! thread (ADR 0035). The serve half ([`serve`], S2.1) is hyper on a thread and runtime of its own
//! per served instance (ADR 0062 decision 5), serving HTTP/1.1 and HTTP/2.

//!
//! The transport is behind the default `transport` feature; without it the crate is the matcher half
//! and what the kind declares ([`kind`]), which need no runtime and build for `wasm32-wasip2`
//! (ADR 0092, ADR 0094).

mod compare;
#[cfg(feature = "transport")]
mod drive;
pub mod kind;
pub mod list;
mod media_type;
pub mod path;
#[cfg(feature = "transport")]
mod serve;
pub mod status;
#[cfg(feature = "transport")]
mod transport;
pub use list::LIST;
pub use media_type::MEDIA_TYPE;
pub use path::PATH;
#[cfg(feature = "transport")]
pub use transport::HttpTransport;
