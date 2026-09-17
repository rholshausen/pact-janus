//! Event streams and `events/poll` (engine-protocol spec §9, ADR 0005): the delivery half of
//! every operation that reports progress rather than a single result. Polling is the baseline on
//! every pipe, so a stream is a buffer with a sequence counter, not a callback.
//!
//! Three properties the spec makes normative and this module makes structural:
//!
//! - **`seq` starts at 1 and never gaps** (§9.2) — the counter lives with the buffer and is
//!   incremented by the only code that can append, so a gap is unrepresentable rather than
//!   merely untested.
//! - **Termination is structural** (§9.2) — the final event carries `last: true` whatever its
//!   kind, and a stream ends when that event has been *delivered* (§9.1), not when it was
//!   produced. Until the host has drained it, the stream id is still valid; after, polling it is
//!   `stream-not-found`.
//! - **Loss is never the pressure valve** (§9.3) — a full buffer blocks the producer
//!   ([`Stream::emit`] waits) instead of dropping or overwriting. The producer runs on its own
//!   thread (the verification run, plan task 5.1), which is what makes blocking it safe: the
//!   dispatch thread stays free to answer the very poll that drains it.

use serde::Serialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

/// How many undelivered events one stream buffers before [`Stream::emit`] starts blocking
/// (spec §9.3's backpressure). Large enough that a host polling at any sane interval never
/// stalls a run, small enough that a host that started work and walked away cannot grow the
/// buffer without bound.
const BUFFER_CAP: usize = 1024;

/// How many events one `events/poll` returns when the host names no `max` ("default: engine's
/// choice", spec §9.4). Chosen so a run's whole event set for a small contract arrives in one
/// call while a large one still pages.
const DEFAULT_MAX: usize = 128;

/// One event (spec §9.2). `last` is always written, not omitted when false: it is the signal a
/// host reads to decide whether the stream is over, and making the reader depend on a default
/// for that is a worse trade than four bytes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
  pub stream: String,
  pub seq: u64,
  pub kind: String,
  pub payload: Value,
  pub last: bool,
}

/// An ordered event buffer for one unit of work (spec §9.1). Shared between the thread producing
/// the events and the dispatch thread draining them, hence the interior mutability: both sides
/// hold the same `Arc<Stream>` and neither owns it.
pub(crate) struct Stream {
  id: String,
  inner: Mutex<Inner>,
  /// Signalled on both edges — a poller waits for events to arrive, a blocked producer waits for
  /// room — so both waits share one condvar and every notification is a `notify_all`.
  change: Condvar,
}

#[derive(Default)]
struct Inner {
  next_seq: u64,
  pending: VecDeque<Event>,
  /// The terminal event has been appended. The producer is done; the host may not be.
  produced_last: bool,
  /// The terminal event has been delivered — from here the stream id is invalid (spec §9.1).
  delivered_last: bool,
}

impl Stream {
  pub fn new(id: impl Into<String>) -> Self {
    Stream {
      id: id.into(),
      inner: Mutex::new(Inner::default()),
      change: Condvar::new(),
    }
  }

  pub fn id(&self) -> &str {
    &self.id
  }

  /// Append an event, blocking while the buffer is full (spec §9.3). Emitting after the terminal
  /// event is a producer bug, not a host-visible condition, so it is dropped with a warning
  /// rather than panicking across a thread boundary where nothing could catch it.
  pub fn emit(&self, kind: impl Into<String>, payload: Value) {
    self.append(kind.into(), payload, false);
  }

  /// Append the terminal event (`last: true`, spec §9.2). The stream does not end here — it ends
  /// when this event is drained.
  pub fn finish(&self, kind: impl Into<String>, payload: Value) {
    self.append(kind.into(), payload, true);
  }

  fn append(&self, kind: String, payload: Value, last: bool) {
    let mut inner = self.inner.lock().expect("stream mutex poisoned");
    if inner.produced_last {
      tracing::warn!(
        stream = %self.id,
        kind = %kind,
        "event emitted after the terminal event; dropped"
      );
      return;
    }
    while inner.pending.len() >= BUFFER_CAP {
      tracing::debug!(stream = %self.id, cap = BUFFER_CAP, "event buffer full, pausing the producer");
      inner = self.change.wait(inner).expect("stream mutex poisoned");
    }
    inner.next_seq += 1;
    let event = Event {
      stream: self.id.clone(),
      seq: inner.next_seq,
      kind,
      payload,
      last,
    };
    tracing::trace!(stream = %self.id, seq = event.seq, kind = %event.kind, "event emitted");
    inner.produced_last = last;
    inner.pending.push_back(event);
    self.change.notify_all();
  }

  /// Whether the terminal event has been delivered — the stream id is spent (spec §9.1), and the
  /// session it belongs to has ended with it (§7.1).
  pub fn ended(&self) -> bool {
    self.inner.lock().expect("stream mutex poisoned").delivered_last
  }

  /// Drain up to `max` events, in `seq` order, without waiting. Returns what was pending; an
  /// empty vector means nothing is pending, which is not an error on any stream (spec §9.4).
  pub fn drain(&self, max: usize) -> Vec<Event> {
    let mut inner = self.inner.lock().expect("stream mutex poisoned");
    let mut drained = Vec::new();
    while drained.len() < max {
      let Some(event) = inner.pending.pop_front() else {
        break;
      };
      if event.last {
        inner.delivered_last = true;
      }
      drained.push(event);
    }
    if !drained.is_empty() {
      // Room freed: whatever producer is parked in `append` can carry on.
      self.change.notify_all();
    }
    drained
  }

  /// Wait up to `timeout` for at least one event to be pending (spec §9.4's long poll). Returns
  /// immediately if events are already pending, or if the producer has finished — waiting past
  /// the terminal event would hold a response open for a stream that will never say more.
  pub fn wait_for_event(&self, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    let mut inner = self.inner.lock().expect("stream mutex poisoned");
    while inner.pending.is_empty() && !inner.produced_last {
      let remaining = deadline.saturating_duration_since(Instant::now());
      if remaining.is_zero() {
        break;
      }
      let (guard, _timed_out) = self
        .change
        .wait_timeout(inner, remaining)
        .expect("stream mutex poisoned");
      inner = guard;
    }
  }
}

/// The `events/poll` request body (spec §9.4). `wait-ms` defaults to 0 — return immediately —
/// so a host that omits it gets the poll semantics every pipe supports.
#[derive(Debug, serde::Deserialize)]
pub struct Poll {
  pub streams: Vec<String>,
  pub max: Option<usize>,
  #[serde(rename = "wait-ms", default)]
  pub wait_ms: u64,
}

impl Poll {
  /// The cap this call applies, clamped to the engine's own default when the host asked for more
  /// (or, meaninglessly, for none).
  pub fn max(&self) -> usize {
    match self.max {
      Some(0) | None => DEFAULT_MAX,
      Some(max) => max.min(BUFFER_CAP),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;
  use std::sync::Arc;

  fn payload(n: u64) -> Value {
    json!({ "n": n })
  }

  #[test]
  fn seq_starts_at_one_and_never_gaps() {
    let stream = Stream::new("s-1");
    for n in 0..5 {
      stream.emit("verification/interaction-result", payload(n));
    }
    let drained = stream.drain(10);
    assert_eq!(
      drained.iter().map(|e| e.seq).collect::<Vec<_>>(),
      vec![1, 2, 3, 4, 5]
    );
    assert!(drained.iter().all(|e| e.stream == "s-1"));
  }

  #[test]
  fn draining_is_ordered_and_resumes_where_it_left_off() {
    let stream = Stream::new("s-1");
    for n in 0..4 {
      stream.emit("verification/interaction-started", payload(n));
    }
    let first = stream.drain(2);
    let second = stream.drain(2);
    assert_eq!(first.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(second.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![3, 4]);
    assert!(stream.drain(2).is_empty());
  }

  #[test]
  fn the_stream_ends_when_the_terminal_event_is_delivered_not_when_it_is_produced() {
    let stream = Stream::new("s-1");
    stream.emit("verification/started", payload(0));
    stream.finish("verification/finished", payload(1));
    assert!(!stream.ended(), "produced, not yet delivered");

    let first = stream.drain(1);
    assert!(!first[0].last);
    assert!(!stream.ended());

    let last = stream.drain(1);
    assert!(last[0].last);
    assert!(stream.ended());
  }

  #[test]
  fn events_after_the_terminal_event_are_dropped() {
    let stream = Stream::new("s-1");
    stream.finish("verification/finished", payload(0));
    stream.emit("verification/hook", payload(1));
    let drained = stream.drain(10);
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].kind, "verification/finished");
  }

  #[test]
  fn a_full_buffer_pauses_the_producer_until_the_host_drains() {
    let stream = Arc::new(Stream::new("s-1"));
    let producer = Arc::clone(&stream);
    let count = BUFFER_CAP + 10;
    let thread = std::thread::spawn(move || {
      for n in 0..count as u64 {
        producer.emit("verification/interaction-result", payload(n));
      }
    });

    let mut seen = Vec::new();
    while seen.len() < count {
      stream.wait_for_event(Duration::from_secs(5));
      seen.extend(stream.drain(64).into_iter().map(|e| e.seq));
    }
    thread.join().expect("producer thread panicked");

    // Nothing was dropped and nothing was reordered, which is the whole claim of §9.3.
    assert_eq!(seen, (1..=count as u64).collect::<Vec<_>>());
  }

  #[test]
  fn a_long_poll_returns_as_soon_as_an_event_arrives() {
    let stream = Arc::new(Stream::new("s-1"));
    let producer = Arc::clone(&stream);
    std::thread::spawn(move || {
      std::thread::sleep(Duration::from_millis(20));
      producer.emit("verification/started", payload(0));
    });
    stream.wait_for_event(Duration::from_secs(5));
    assert_eq!(stream.drain(10).len(), 1);
  }

  #[test]
  fn a_long_poll_on_a_finished_producer_does_not_wait_out_its_timeout() {
    let stream = Stream::new("s-1");
    stream.finish("verification/finished", payload(0));
    assert_eq!(stream.drain(10).len(), 1);
    let started = Instant::now();
    stream.wait_for_event(Duration::from_secs(30));
    assert!(
      started.elapsed() < Duration::from_secs(1),
      "returned without waiting"
    );
  }

  #[test]
  fn poll_bodies_clamp_max_to_the_engines_own_choice() {
    let none: Poll = serde_json::from_value(json!({ "streams": ["s-1"] })).unwrap();
    assert_eq!(none.max(), DEFAULT_MAX);
    assert_eq!(none.wait_ms, 0);

    let asked: Poll = serde_json::from_value(json!({ "streams": ["s-1"], "max": 3 })).unwrap();
    assert_eq!(asked.max(), 3);

    let absurd: Poll =
      serde_json::from_value(json!({ "streams": ["s-1"], "max": 10_000, "wait-ms": 50 })).unwrap();
    assert_eq!(absurd.max(), BUFFER_CAP);
    assert_eq!(absurd.wait_ms, 50);
  }

  #[test]
  fn an_event_serializes_as_the_schemas_shape() {
    let stream = Stream::new("s-1");
    stream.finish("verification/finished", json!({ "status": "verified" }));
    let event = &stream.drain(1)[0];
    assert_eq!(
      serde_json::to_value(event).unwrap(),
      json!({
        "stream": "s-1",
        "seq": 1,
        "kind": "verification/finished",
        "payload": { "status": "verified" },
        "last": true
      })
    );
  }
}
