//! The Janus engine as a WASM component: the embedding for offline operations (ADR 0023
//! decision 2), for a host that wants Janus's answers without a native binary.
//!
//! It is `Engine::dispatch` behind the frozen pipe, with the in-tree JSON content component, and it
//! has no transports: a `wasm32-wasip2` guest has sockets, but no threads for the exchange loop or
//! the HTTP transport's server, and that transport was not built for the target (Phase 9 finding 3).
//! So it does what the kernel does without I/O (handshake, compile, variant enumeration, upgrade,
//! subsumption), and `start-transport` and `verification/verify` fail by name, as
//! `component-unavailable`.
//!
//! Plan task 9.1 built it to measure it, in `benchmarks/janus/`; staged plan S0.4 moved it here and
//! ships it. The harness still measures it, through the `bench` feature's second world.
//!
//! A component's export names are not symbols a native linker accepts, so on any other target this
//! crate is empty: the workspace's host build passes over it, and what there is to check is checked
//! with `--target wasm32-wasip2`.
#![cfg(target_arch = "wasm32")]

use pact_janus_component_form::FormContent;
use pact_janus_component_json::JsonContent;
use pact_janus_component_text::TextContent;
use pact_janus_component_xml::XmlContent;
use pact_janus_kernel::protocol::Engine;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(not(feature = "bench"))]
wit_bindgen::generate!({ world: "engine", path: "wit" });
#[cfg(feature = "bench")]
wit_bindgen::generate!({ world: "engine-bench", path: "wit" });

thread_local! {
  static ENGINE: RefCell<Engine> = RefCell::new(engine());
}

fn engine() -> Engine {
  let mut engine = Engine::with_components(HashMap::new(), Some(Arc::new(JsonContent::new())));
  engine.declare_in_tree("content", "json", "1.0.0");
  // XML before text, which claims all of `text/*`, so `text/xml` reaches it (ADR 0039).
  engine.register_content(Arc::new(XmlContent::new()));
  engine.declare_in_tree("content", "xml", "1.0.0");
  engine.register_content(Arc::new(TextContent::new()));
  engine.declare_in_tree("content", "text", "1.0.0");
  engine.register_content(Arc::new(FormContent::new()));
  engine.declare_in_tree("content", "form", "1.0.0");
  engine
}

struct Guest;

impl exports::pact::janus_engine::pipe::Guest for Guest {
  fn call(request: Vec<u8>) -> Vec<u8> {
    ENGINE.with(|engine| engine.borrow_mut().dispatch(&request))
  }
}

#[cfg(feature = "bench")]
mod bench {
  use super::{Guest, exports};
  use pact_janus_kernel::plan::{self, CapturedValues, Plan};
  use std::cell::RefCell;
  use std::collections::BTreeMap;

  thread_local! {
    static PLAN: RefCell<Option<Plan>> = const { RefCell::new(None) };
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
}

export!(Guest);
