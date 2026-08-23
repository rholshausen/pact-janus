//! Native Rust engine hosting WASM component plugins (spike 1.4).
//!
//! Scenarios: discovery handshake; plan execution mixing core + plugin
//! actions; sandbox probes; panic containment; epoch-based runaway
//! termination; per-invocation benchmarks.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi::ResourceTable;

wasmtime::component::bindgen!({
    path: "../wit",
    world: "plugin",
});

struct Ctx {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl IoView for Ctx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
impl WasiView for Ctx {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

/// A loaded plugin: component + a live instance in its own store.
struct LoadedPlugin {
    name: String,
    actions: Vec<String>,
    store: Store<Ctx>,
    instance: Plugin,
}

const NORMAL_DEADLINE_TICKS: u64 = 1_000; // ~10 s at 10 ms/tick
const SPIN_DEADLINE_TICKS: u64 = 20; // ~200 ms

fn locked_down_store(engine: &Engine) -> Store<Ctx> {
    // The sandbox: no preopened dirs, no env, no args, no sockets. Only
    // stderr is granted (so panic messages surface somewhere visible).
    let wasi = WasiCtxBuilder::new().inherit_stderr().build();
    let mut store = Store::new(engine, Ctx { wasi, table: ResourceTable::new() });
    store.set_epoch_deadline(NORMAL_DEADLINE_TICKS);
    store
}

fn instantiate(engine: &Engine, linker: &Linker<Ctx>, component: &Component) -> Result<(Store<Ctx>, Plugin)> {
    let mut store = locked_down_store(engine);
    let instance = Plugin::instantiate(&mut store, component, linker)?;
    Ok((store, instance))
}

fn call(store: &mut Store<Ctx>, instance: &Plugin, frame: &Value) -> Result<Value> {
    let request = serde_json::to_vec(frame)?;
    store.set_epoch_deadline(NORMAL_DEADLINE_TICKS);
    let response = instance.pact_plugin_pipe().call_call(&mut *store, &request)?;
    Ok(serde_json::from_slice(&response)?)
}

fn load_plugin(engine: &Engine, linker: &Linker<Ctx>, component: &Component) -> Result<LoadedPlugin> {
    let (mut store, instance) = instantiate(engine, linker, component)?;
    let hs = call(&mut store, &instance, &json!({ "op": "handshake", "protocol-versions": [1] }))?;
    let ok = hs.get("ok").ok_or_else(|| anyhow!("handshake failed: {hs}"))?;
    Ok(LoadedPlugin {
        name: ok["name"].as_str().unwrap_or("?").to_string(),
        actions: ok["actions"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        store,
        instance,
    })
}

// --- The toy plan executor: core actions inline, plugin: actions dispatched.

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// The 1.2 toy structural type matcher, natively in the "kernel".
fn match_type(expected: &Value, actual: &Value, path: &str, mismatches: &mut Vec<Value>) {
    match (expected, actual) {
        (Value::Object(exp), Value::Object(act)) => {
            for (key, exp_child) in exp {
                let child = format!("{path}.{key}");
                match act.get(key) {
                    Some(act_child) => match_type(exp_child, act_child, &child, mismatches),
                    None => mismatches.push(json!({ "path": child, "expected": "present", "actual": "missing" })),
                }
            }
        }
        (Value::Array(exp), Value::Array(act)) => {
            if let Some(template) = exp.first() {
                for (i, act_child) in act.iter().enumerate() {
                    match_type(template, act_child, &format!("{path}[{i}]"), mismatches);
                }
            }
        }
        _ => {
            if type_name(expected) != type_name(actual) {
                mismatches.push(json!({ "path": path, "expected": type_name(expected), "actual": type_name(actual) }));
            }
        }
    }
}

fn execute_plan(plan: &Value, plugins: &mut [LoadedPlugin], registry: &HashMap<String, usize>) -> Value {
    let mut node_results = Vec::new();
    for node in plan["nodes"].as_array().unwrap_or(&Vec::new()) {
        let action = node["action"].as_str().unwrap_or("");
        let result = if let Some(core_action) = action.strip_prefix("core:") {
            match core_action {
                "match-type" => {
                    let mut mismatches = Vec::new();
                    match_type(&node["expected"], &node["actual"], "$", &mut mismatches);
                    json!({ "executed-by": "kernel", "matched": mismatches.is_empty(), "mismatches": mismatches })
                }
                other => json!({ "error": { "code": "unknown-core-action", "action": other } }),
            }
        } else if let Some(plugin_action) = action.strip_prefix("plugin:") {
            match registry.get(plugin_action).map(|i| &mut plugins[*i]) {
                Some(plugin) => {
                    let mut frame = node.clone();
                    frame["op"] = json!("invoke");
                    frame["action"] = json!(plugin_action);
                    match call(&mut plugin.store, &plugin.instance, &frame) {
                        Ok(resp) => match resp.get("ok") {
                            Some(ok) => {
                                let mut r = ok.clone();
                                r["executed-by"] = json!(format!("plugin '{}'", plugin.name));
                                r
                            }
                            None => json!({ "error": resp.get("err").cloned().unwrap_or(Value::Null) }),
                        },
                        Err(e) => json!({ "error": { "code": "plugin-call-failed", "detail": e.to_string() } }),
                    }
                }
                None => json!({ "error": { "code": "no-plugin-for-action", "action": plugin_action } }),
            }
        } else {
            json!({ "error": { "code": "unknown-action-namespace", "action": action } })
        };
        node_results.push(json!({ "action": action, "result": result }));
    }
    json!({ "nodes": node_results })
}

fn stats(name: &str, samples: &mut [u128]) {
    samples.sort_unstable();
    let at = |q: f64| samples[((q * samples.len() as f64) as usize).min(samples.len() - 1)] as f64 / 1e3;
    println!("  {name}: median {:.1}µs  p95 {:.1}µs  min {:.1}µs", at(0.5), at(0.95), samples[0] as f64 / 1e3);
}

fn bench(name: &str, warmup: usize, iters: usize, mut f: impl FnMut()) {
    for _ in 0..warmup {
        f();
    }
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        f();
        samples.push(t0.elapsed().as_nanos());
    }
    stats(name, &mut samples);
}

fn main() -> Result<()> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;

    // Epoch ticker: 10 ms per tick, for the lifetime of the process.
    {
        let engine = engine.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(10));
            engine.increment_epoch();
        });
    }

    let mut linker: Linker<Ctx> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

    println!("scenario: load + compile");
    let t0 = Instant::now();
    let matcher_component = Component::from_file(&engine, "../plugins/target/wasm32-wasip1/release/plugin_matcher.wasm")?;
    println!("  matcher compile: {:.1}ms", t0.elapsed().as_secs_f64() * 1e3);
    let t0 = Instant::now();
    let misbehaved_component =
        Component::from_file(&engine, "../plugins/target/wasm32-wasip1/release/plugin_misbehaved.wasm")?;
    println!("  misbehaved compile: {:.1}ms", t0.elapsed().as_secs_f64() * 1e3);

    println!("scenario: discovery handshake");
    let mut matcher = load_plugin(&engine, &linker, &matcher_component)?;
    println!("  plugin '{}' contributes actions {:?}", matcher.name, matcher.actions);
    assert!(matcher.actions.contains(&"match:luhn".to_string()));

    println!("scenario: plan execution (core + plugin actions)");
    {
        let plan = json!({ "nodes": [
            { "action": "core:match-type",
              "expected": { "card": { "number": "4539578763621486" }, "amount": 12.5 },
              "actual":   { "card": { "number": "4539578763621486" }, "amount": "12.50" } },
            { "action": "plugin:match:luhn", "actual": "4539578763621486" },
            { "action": "plugin:match:luhn", "actual": "4539578763621487" },
            { "action": "plugin:match:unknown", "actual": "x" },
        ]});
        let mut plugins = vec![matcher];
        let registry: HashMap<String, usize> =
            plugins[0].actions.iter().map(|a| (a.clone(), 0)).collect();
        let result = execute_plan(&plan, &mut plugins, &registry);
        matcher = plugins.pop().unwrap();
        println!("{}", serde_json::to_string_pretty(&result)?);
        assert_eq!(result["nodes"][0]["result"]["matched"], json!(false)); // number vs string
        assert_eq!(result["nodes"][1]["result"]["matched"], json!(true));
        assert_eq!(result["nodes"][2]["result"]["matched"], json!(false));
        assert_eq!(result["nodes"][3]["result"]["error"]["code"], json!("no-plugin-for-action"));
    }

    println!("scenario: sandbox probes (locked-down WASI)");
    let mut misbehaved = load_plugin(&engine, &linker, &misbehaved_component)?;
    {
        let r = call(&mut misbehaved.store, &misbehaved.instance, &json!({ "op": "read-file", "path": "/etc/passwd" }))?;
        println!("  read /etc/passwd -> {r}");
        assert!(r.get("err").is_some(), "fs access must be denied");
        let r = call(&mut misbehaved.store, &misbehaved.instance, &json!({ "op": "read-file", "path": "Cargo.toml" }))?;
        println!("  read ./Cargo.toml -> {r}");
        assert!(r.get("err").is_some(), "fs access must be denied");
        let r = call(&mut misbehaved.store, &misbehaved.instance, &json!({ "op": "get-env" }))?;
        println!("  get-env -> {r}");
        assert_eq!(r["ok"]["var-count"], json!(0), "environment must be empty");
    }

    println!("scenario: panic containment");
    {
        let result = call(&mut misbehaved.store, &misbehaved.instance, &json!({ "op": "panic" }));
        match &result {
            Err(e) => println!("  panic surfaced to host as error: {}", first_line(&format!("{e:?}"))),
            Ok(v) => println!("  UNEXPECTED ok: {v}"),
        }
        assert!(result.is_err());
        // Is the instance usable after a trap?
        let after = call(&mut misbehaved.store, &misbehaved.instance, &json!({ "op": "get-env" }));
        match &after {
            Err(e) => println!("  call after trap on same instance: refused ({})", first_line(&format!("{e}"))),
            Ok(v) => println!("  call after trap on same instance: worked ({v})"),
        }
        // Recovery: fresh instance from the same compiled component.
        let t0 = Instant::now();
        misbehaved = load_plugin(&engine, &linker, &misbehaved_component)?;
        let r = call(&mut misbehaved.store, &misbehaved.instance, &json!({ "op": "get-env" }))?;
        assert!(r.get("ok").is_some());
        println!("  re-instantiated + handshake in {:.1}ms; plugin healthy again", t0.elapsed().as_secs_f64() * 1e3);
    }

    println!("scenario: runaway plugin (epoch deadline)");
    {
        misbehaved.store.set_epoch_deadline(SPIN_DEADLINE_TICKS);
        let t0 = Instant::now();
        let request = serde_json::to_vec(&json!({ "op": "spin" }))?;
        let result = misbehaved.instance.pact_plugin_pipe().call_call(&mut misbehaved.store, &request);
        let elapsed = t0.elapsed();
        match &result {
            Err(e) => println!(
                "  spin terminated by engine after {:.0}ms (deadline ~200ms): {}",
                elapsed.as_secs_f64() * 1e3,
                first_line(&format!("{e}"))
            ),
            Ok(_) => println!("  UNEXPECTED: spin returned"),
        }
        assert!(result.is_err());
        assert!(elapsed < Duration::from_secs(2), "deadline must bound the call");
        misbehaved = load_plugin(&engine, &linker, &misbehaved_component)?;
        let _ = &misbehaved;
    }

    println!("scenario: benchmarks");
    {
        let valid = "4539578763621486";
        // Native baseline: same Luhn logic compiled into the host.
        bench("luhn native", 500, 5000, || {
            std::hint::black_box(luhn_valid_native(std::hint::black_box(valid)));
        });
        let frame = json!({ "op": "invoke", "action": "match:luhn", "actual": valid });
        bench("luhn via plugin (incl. JSON frame)", 500, 5000, || {
            call(&mut matcher.store, &matcher.instance, &frame).unwrap();
        });

        let doc: Value = serde_json::from_str(&std::fs::read_to_string(
            "../../1.2-wasm-embedding/payloads/order-100kb.json",
        )?)?;
        let frame_100k = json!({ "op": "echo", "payload": doc });
        bench("echo-100k via plugin", 50, 500, || {
            call(&mut matcher.store, &matcher.instance, &frame_100k).unwrap();
        });

        // Instantiation costs: cold vs pre-instantiated.
        let instance_pre = linker.instantiate_pre(&matcher_component)?;
        bench("instantiate (Linker::instantiate)", 20, 200, || {
            instantiate(&engine, &linker, &matcher_component).unwrap();
        });
        bench("instantiate (InstancePre)", 20, 200, || {
            let mut store = locked_down_store(&engine);
            instance_pre.instantiate(&mut store).unwrap();
        });
    }

    println!("all scenarios passed");
    Ok(())
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

fn luhn_valid_native(s: &str) -> bool {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let sum: u32 = s
        .chars()
        .rev()
        .filter_map(|c| c.to_digit(10))
        .enumerate()
        .map(|(i, d)| if i % 2 == 1 { if d * 2 > 9 { d * 2 - 9 } else { d * 2 } } else { d })
        .sum();
    sum % 10 == 0
}
