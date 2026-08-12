//! protobuf gauntlet leg: E1–E5 as encoding-level evolution between two compiled views of
//! the same proto package. "old" = the plugin's view (v1), "new" = the engine's view (v2).
//! No gRPC — frames as they would travel over any pipe (stdio, byte-pipe WIT world, C ABI).
use anyhow::Result;
use prost::Message;

mod v1 {
  include!(concat!(env!("OUT_DIR"), "/v1/pact.engine.slice.rs"));
}
mod v2 {
  include!(concat!(env!("OUT_DIR"), "/v2/pact.engine.slice.rs"));
}

fn doc(s: &str) -> v2::Document {
  v2::Document { json: s.as_bytes().to_vec() }
}

fn main() -> Result<()> {
  // ── E1: new enum value, D-a (new engine writes, old plugin reads) ───────────────
  let bytes = v2::EngineError {
    code: v2::ErrorCode::ComponentUnavailable as i32,
    message: "component x unavailable".into(),
    details: None,
    retryable: Some(true), // E2 rides along
  }
  .encode_to_vec();
  let old = v1::EngineError::decode(&bytes[..])?;
  println!("E1 D-a: old decode OK; raw code = {} (wire value survives)", old.code);
  println!(
    "E1 D-a: typed accessor sees {:?} — SILENT fallback, no unknown-value signal",
    old.code()
  );
  let detectable = v1::ErrorCode::try_from(old.code).is_err();
  println!("E1 D-a: detectable by hand via try_from on raw i32: {detectable}");

  // ── E2: new optional field, D-a + pass-through round-trip ───────────────────────
  println!("E2 D-a: old decode OK; new field invisible to old view");
  let round_tripped = v2::EngineError::decode(&old.encode_to_vec()[..])?;
  println!(
    "E2 pass-through: retryable after old-view round-trip = {:?} — prost DROPS unknown \
     fields (protobuf-java/C++ would preserve them)",
    round_tripped.retryable
  );
  println!(
    "E2 pass-through: enum value after round-trip = {:?} (out-of-range enum survives; \
     it is a known field)",
    round_tripped.code()
  );

  // ── E4: new oneof case in the event stream, D-a ──────────────────────────────────
  let ev_bytes = v2::VerifyEvent {
    event: Some(v2::verify_event::Event::HookInvoked(doc("before-request"))),
  }
  .encode_to_vec();
  let old_ev = v1::VerifyEvent::decode(&ev_bytes[..])?;
  println!(
    "E4 D-a: old decode OK; event = {:?} — unknown case arrives as None, \
     indistinguishable from an empty event",
    old_ev.event
  );

  // ── E5: new alternative in a payload union, D-a ──────────────────────────────────
  let part_bytes = v2::InteractionPart {
    part: Some(v2::interaction_part::Part::Message(doc("{}"))),
  }
  .encode_to_vec();
  let old_part = v1::InteractionPart::decode(&part_bytes[..])?;
  println!("E5 D-a: old decode OK; part = {:?} — same silent-None shape as E4", old_part.part);

  // ── E3: new operation, D-a (new caller, old callee) ──────────────────────────────
  let req_bytes = v2::Request {
    op: Some(v2::request::Op::Explain(v2::Explain { session: 1, spec: Some(doc("{}")) })),
  }
  .encode_to_vec();
  let old_req = v1::Request::decode(&req_bytes[..])?;
  match old_req.op {
    None => println!(
      "E3 D-a: old callee sees op = None and can answer 'unsupported operation' — \
       explicit rejection is codeable, unlike a link error"
    ),
    Some(op) => println!("E3 D-a: UNEXPECTED known op {op:?}"),
  }

  // ── D-b: old writer, new reader (old plugin talks to new engine) ─────────────────
  let old_bytes = v1::EngineError {
    code: v1::ErrorCode::Internal as i32,
    message: "boom".into(),
    details: None,
  }
  .encode_to_vec();
  let new = v2::EngineError::decode(&old_bytes[..])?;
  println!(
    "D-b: new decode of old frame OK; code = {:?}, retryable = {:?} (absent -> None)",
    new.code(),
    new.retryable
  );
  let old_ev_bytes =
    v1::VerifyEvent { event: Some(v1::verify_event::Event::Started(v1::Document {
      json: b"{}".to_vec(),
    })) }
    .encode_to_vec();
  let new_ev = v2::VerifyEvent::decode(&old_ev_bytes[..])?;
  println!("D-b: old event under new view = known case {:?}", new_ev.event.is_some());

  println!("RESULT: OK");
  Ok(())
}
