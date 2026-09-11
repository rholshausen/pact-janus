//! Parts, slots and their values (component-interfaces spec §4, `schemas/v1/parts.schema.json`):
//! "part name -> slot name -> slot value." The spec says this is design 2.5's `SlotValue`
//! unchanged — "the same document crosses both boundaries by intent, so nothing translates
//! between them" — and it is, field for field ([`crate::contract::SlotValue`]: `content`,
//! `encoded`, `content-type`), so it is reused rather than redefined here.

use std::collections::BTreeMap;

pub use crate::contract::SlotValue;

/// One part: slot name -> slot value. Slot names are the transport's and content component's
/// business, never the kernel's (spec §4) — this is why it's a plain map, not a struct.
pub type Part = BTreeMap<String, SlotValue>;

/// Part name -> part (e.g. `"request"`/`"response"` for HTTP, but that pairing is not fixed here —
/// a message interaction has neither).
pub type Parts = BTreeMap<String, Part>;
