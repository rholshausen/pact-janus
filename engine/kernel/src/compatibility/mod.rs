//! The compatibility decision (plan task 7.4): one subsumption report per pair, plus whatever
//! verification results the host has, plus a policy, turned into the RFC's `can-i-deploy` answer.
//!
//! This is the *page* design 2.8 deliberately does not define. Its §6.4 fixes the finding block
//! and says so in as many words — "combining this block with verification-result lines into one
//! `can-i-deploy` report is task 7.4's job; this specification fixes the block, not the page it
//! appears on" — and its §1 lists the combination as out of scope with this task as the owner. So
//! the document this module writes is engine-protocol spec §8.6's, and everything it does to a
//! finding on the way there is design 2.8 §7's.
//!
//! Three separations are load-bearing, and all three are one rule: **nothing here re-decides
//! anything.**
//!
//! 1. **A verdict is the walk's, a decision is the policy's.** An exempted finding is still a
//!    `no`, and the pair's `subsumption.verdict` still says `no` — what changes is the decision
//!    over it. A report that rewrote the verdict would destroy the only record of what the
//!    checker actually found, which is what a team revisiting an exemption needs to read.
//! 2. **Verification is not policy-governed.** `on-finding`/`on-review` resolve what a
//!    *subsumption* severity does (design 2.8 §7.1). A verification that ran and found mismatches
//!    is a decided incompatibility on the evidence, and no exemption list can make it a pass:
//!    that is the build failing, and it blocks.
//! 3. **A missing input is never a pass.** No verification result for a pair warns, an aborted
//!    run blocks, and a filtered run says so — the same rule `janus verify` already applies to
//!    its own summary ("This was a filtered run: unreplayed variants are not passing ones").
//!    A provider that published no shape is the one exception, and not an omission: the RFC makes
//!    that replay-only semantics, which is what makes the mechanism adoptable per-provider.

mod decide;
mod render;

pub use decide::{
  CompatibilityReport, Decision, Disposition, ExemptionRef, ExemptionResult, ExemptionStatus, FORMAT,
  FindingEntry, InteractionId, Pair, PairResult, PolicyView, Reason, ReasonAction, SubsumptionView, Summary,
  VerificationView, decide,
};
pub use render::render;
