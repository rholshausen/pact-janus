//! The Janus engine as a WASM component (plan task 9.1).
//!
//! ADR 0003 decided that one engine build produces three artifacts, with the WASM component as the
//! canonical one. Until this task only the subprocess one existed. This crate is the smallest
//! honest version of the component: `Engine::dispatch` behind the frozen pipe, with the in-tree
//! JSON content component. It has no transports: a `wasm32-wasip2` guest has sockets, but no threads
//! for the exchange loop or the HTTP transport's server, and that transport was not built for the
//! target (Phase 9 finding 3). So it can do what the kernel does without I/O (handshake, compile, variant
//! enumeration, upgrade, subsumption), and `start-transport` and `verification/verify` fail by
//! name.

use pact_janus_component_json::JsonContent;
use pact_janus_kernel::plan::{self, CapturedValues, Plan};
use pact_janus_kernel::protocol::Engine;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

wit_bindgen::generate!({ world: "engine", path: "wit" });

thread_local! {
  static ENGINE: RefCell<Engine> = RefCell::new(engine());
  static PLAN: RefCell<Option<Plan>> = const { RefCell::new(None) };
}

fn engine() -> Engine {
  let mut engine = Engine::with_components(HashMap::new(), Some(Arc::new(JsonContent::new())));
  engine.declare_in_tree("content", "json", "1.0.0");
  engine
}

struct Guest;

impl exports::pact::janus_engine::pipe::Guest for Guest {
  fn call(request: Vec<u8>) -> Vec<u8> {
    ENGINE.with(|engine| engine.borrow_mut().dispatch(&request))
  }
}

impl exports::pact::janus_engine::bench::Guest for Guest {
  fn set_plan(document: Vec<u8>) -> Result<(), String> {
    let document: serde_json::Value = serde_json::from_slice(&document).map_err(|err| err.to_string())?;
    let compiled = plan::from_json(&document)?;
    PLAN.with(|slot| *slot.borrow_mut() = Some(compiled));
    Ok(())
  }

  fn execute(values: Vec<u8>) -> Result<String, String> {
    let values: BTreeMap<String, serde_json::Value> =
      serde_json::from_slice(&values).map_err(|err| err.to_string())?;
    PLAN.with(|slot| {
      let slot = slot.borrow();
      let compiled = slot.as_ref().ok_or("set-plan first")?;
      let executed = plan::execute(compiled, &CapturedValues::from_json(&values));
      Ok(match plan::outcome(&executed).0 {
        plan::Status::Matched => "matched".to_string(),
        plan::Status::Mismatched => "mismatched".to_string(),
      })
    })
  }
}

export!(Guest);
