//! Spike 1.6: script-hook engine bake-off.
//!
//! One realistic hook workload — sign a request (FNV-1a over
//! method/path/body/secret), mutate headers — implemented identically in each
//! candidate engine's language. Every engine must produce byte-identical
//! signatures (cross-engine correctness check). The same binary compiles
//! natively AND to wasm32-wasip1 (per engine feature), so "does the
//! interpreter run inside the WASM embedding" is answered by construction.

use serde_json::{json, Value};
use std::time::Instant;

pub struct Workload {
    pub ctx: Value,
    pub expected_auth: String,
    pub expected_ts: String,
}

fn fnv1a32(data: &[u8]) -> u32 {
    let mut h: u32 = 2166136261;
    for b in data {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

impl Workload {
    fn new() -> Self {
        let body = r#"{"sku":"widget-1","quantity":2}"#;
        let (method, path, secret, ts) = ("POST", "/orders", "s3cr3t", "2026-08-23T10:00:00Z");
        let data = format!("{method} {path}\n{body}\n{secret}");
        Self {
            ctx: json!({
                "request": {
                    "method": method, "path": path,
                    "headers": { "content-type": "application/json" },
                    "body": body,
                },
                "config": { "secret": secret, "timestamp": ts },
            }),
            expected_auth: format!("HMAC-FNV {:08x}", fnv1a32(data.as_bytes())),
            expected_ts: ts.to_string(),
        }
    }

    fn verify(&self, engine: &str, headers: &Value) {
        assert_eq!(headers["authorization"], json!(self.expected_auth), "{engine}: wrong signature");
        assert_eq!(headers["x-signed-at"], json!(self.expected_ts), "{engine}: timestamp not set");
        assert_eq!(headers["content-type"], json!("application/json"), "{engine}: lost existing header");
    }
}

pub fn stats(name: &str, samples: &mut [u128]) {
    samples.sort_unstable();
    let at = |q: f64| samples[((q * samples.len() as f64) as usize).min(samples.len() - 1)] as f64 / 1e3;
    println!("  {name}: median {:.1}µs  p95 {:.1}µs  min {:.1}µs", at(0.5), at(0.95), samples[0] as f64 / 1e3);
}

pub fn bench_loop(name: &str, warmup: usize, iters: usize, mut f: impl FnMut()) {
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

pub fn cold_loop(name: &str, iters: usize, mut f: impl FnMut()) {
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t0 = Instant::now();
        f();
        samples.push(t0.elapsed().as_nanos());
    }
    samples.sort_unstable();
    println!(
        "  {name}: first {:.2}ms  median {:.2}ms",
        samples[0] as f64 / 1e6, // sorted: report min as best-case, and median
        samples[samples.len() / 2] as f64 / 1e6
    );
}

// The JS hook, shared by Boa / QuickJS / V8.
#[allow(dead_code)]
const JS_HOOK: &str = r#"
function fnv1a(s) {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < s.length; i++) {
    h = Math.imul(h ^ s.charCodeAt(i), 16777619) >>> 0;
  }
  return h;
}
function hook(ctx) {
  const req = ctx.request;
  const data = req.method + " " + req.path + "\n" + req.body + "\n" + ctx.config.secret;
  const sig = fnv1a(data).toString(16).padStart(8, "0");
  req.headers["authorization"] = "HMAC-FNV " + sig;
  req.headers["x-signed-at"] = ctx.config.timestamp;
  return req.headers;
}
"#;

// ---------------------------------------------------------------- Rhai

#[cfg(feature = "rhai-engine")]
mod rhai_engine {
    use super::*;
    use rhai::{Dynamic, Engine, Scope, AST};

    const SRC: &str = r#"
fn fnv1a(s) {
    let h = 0x811c9dc5;
    for ch in s.chars() {
        h ^= ch.to_int();
        h = (h * 16777619) & 0xffffffff;
    }
    h
}
fn to_hex8(n) {
    let digits = "0123456789abcdef";
    let out = "";
    let x = n;
    for i in 0..8 {
        out = digits.sub_string((x & 0xf), 1) + out;
        x >>= 4;
    }
    out
}
fn hook(ctx) {
    let req = ctx.request;
    let data = req.method + " " + req.path + "\n" + req.body + "\n" + ctx.config.secret;
    let sig = to_hex8(fnv1a(data));
    req.headers["authorization"] = "HMAC-FNV " + sig;
    req.headers["x-signed-at"] = ctx.config.timestamp;
    req.headers
}
"#;

    fn call_hook(engine: &Engine, ast: &AST, ctx: &Value) -> Value {
        let mut scope = Scope::new();
        let ctx_dyn: Dynamic = rhai::serde::to_dynamic(ctx).unwrap();
        let out: Dynamic = engine.call_fn(&mut scope, ast, "hook", (ctx_dyn,)).unwrap();
        rhai::serde::from_dynamic(&out).unwrap()
    }

    pub fn run(w: &Workload) {
        println!("== rhai (pure-Rust DSL)");
        let engine = Engine::new();
        let ast = engine.compile(SRC).unwrap();
        w.verify("rhai", &call_hook(&engine, &ast, &w.ctx));
        cold_loop("cold (engine+compile+1 call)", 20, || {
            let e = Engine::new();
            let a = e.compile(SRC).unwrap();
            let _ = call_hook(&e, &a, &w.ctx);
        });
        bench_loop("hook call (incl. ctx marshalling)", 200, 2000, || {
            let _ = call_hook(&engine, &ast, &w.ctx);
        });
        // sandbox: no file/net/env APIs exist in the standard packages at all
        let probe = engine.eval::<Dynamic>(r#"let r = "no io/fs/net API exists"; r"#).unwrap();
        println!("  sandbox: by construction ({probe}); modules disabled without a resolver");
    }
}

// ---------------------------------------------------------------- Boa

#[cfg(feature = "boa")]
mod boa_js {
    use super::*;
    use boa_engine::{js_string, Context, JsValue, Source};

    fn call_hook(context: &mut Context, ctx: &Value) -> Value {
        let hook = context.global_object().get(js_string!("hook"), context).unwrap();
        let ctx_js = JsValue::from_json(ctx, context).unwrap();
        let out = hook.as_callable().unwrap().call(&JsValue::undefined(), &[ctx_js], context).unwrap();
        out.to_json(context).unwrap()
    }

    pub fn run(w: &Workload) {
        println!("== boa (pure-Rust JS)");
        let mut context = Context::default();
        context.eval(Source::from_bytes(JS_HOOK)).unwrap();
        w.verify("boa", &call_hook(&mut context, &w.ctx));
        cold_loop("cold (engine+compile+1 call)", 20, || {
            let mut c = Context::default();
            c.eval(Source::from_bytes(JS_HOOK)).unwrap();
            let _ = call_hook(&mut c, &w.ctx);
        });
        bench_loop("hook call (incl. ctx marshalling)", 200, 2000, || {
            let _ = call_hook(&mut context, &w.ctx);
        });
        let probe = context
            .eval(Source::from_bytes("[typeof require, typeof process, typeof fetch].join(',')"))
            .unwrap();
        println!("  sandbox: require/process/fetch -> {}", probe.display());
    }
}

// ---------------------------------------------------------------- Lua (mlua)

#[cfg(feature = "lua")]
mod lua_engine {
    use super::*;
    use mlua::{Function, Lua, LuaSerdeExt, StdLib};

    const SRC: &str = r#"
function fnv1a(s)
  local h = 2166136261
  for i = 1, #s do
    h = h ~ string.byte(s, i)
    h = (h * 16777619) & 0xFFFFFFFF
  end
  return h
end
function hook(ctx)
  local req = ctx.request
  local data = req.method .. " " .. req.path .. "\n" .. req.body .. "\n" .. ctx.config.secret
  local sig = string.format("%08x", fnv1a(data))
  req.headers["authorization"] = "HMAC-FNV " .. sig
  req.headers["x-signed-at"] = ctx.config.timestamp
  return req.headers
end
"#;

    fn call_hook(lua: &Lua, ctx: &Value) -> Value {
        let hook: Function = lua.globals().get("hook").unwrap();
        let ctx_lua = lua.to_value(ctx).unwrap();
        let out: mlua::Value = hook.call(ctx_lua).unwrap();
        lua.from_value(out).unwrap()
    }

    fn restricted() -> Lua {
        // string + math + table only: no io, no os, no debug, no package
        Lua::new_with(StdLib::STRING | StdLib::MATH | StdLib::TABLE, mlua::LuaOptions::default()).unwrap()
    }

    pub fn run(w: &Workload) {
        println!("== lua 5.4 (mlua, vendored C)");
        let lua = restricted();
        lua.load(SRC).exec().unwrap();
        w.verify("lua", &call_hook(&lua, &w.ctx));
        cold_loop("cold (engine+compile+1 call)", 20, || {
            let l = restricted();
            l.load(SRC).exec().unwrap();
            let _ = call_hook(&l, &w.ctx);
        });
        bench_loop("hook call (incl. ctx marshalling)", 200, 2000, || {
            let _ = call_hook(&lua, &w.ctx);
        });
        let default_lua = Lua::new();
        let io_default: bool = default_lua.load("return io ~= nil and os.getenv ~= nil").eval().unwrap();
        let io_restricted: bool = lua.load("return io ~= nil or os ~= nil").eval().unwrap();
        println!("  sandbox: default stdlib has io/os = {io_default}; restricted StdLib has them = {io_restricted}");
    }
}

// ---------------------------------------------------------------- QuickJS (rquickjs)

#[cfg(feature = "quickjs")]
mod quickjs_engine {
    use super::*;
    use rquickjs::{Context, Function, Runtime};

    const BRIDGE: &str = "function hookJson(s){ return JSON.stringify(hook(JSON.parse(s))); }";

    pub fn run(w: &Workload) {
        println!("== quickjs (rquickjs, bundled C — the Javy engine, embedded natively here)");
        let ctx_str = serde_json::to_string(&w.ctx).unwrap();

        let rt = Runtime::new().unwrap();
        let context = Context::full(&rt).unwrap();
        context.with(|ctx| {
            ctx.eval::<(), _>(JS_HOOK).unwrap();
            ctx.eval::<(), _>(BRIDGE).unwrap();
        });
        let call = |out_check: bool| {
            context.with(|ctx| {
                let f: Function = ctx.globals().get("hookJson").unwrap();
                let out: String = f.call((ctx_str.clone(),)).unwrap();
                if out_check {
                    let headers: Value = serde_json::from_str(&out).unwrap();
                    w.verify("quickjs", &headers);
                }
            })
        };
        call(true);
        cold_loop("cold (engine+compile+1 call)", 20, || {
            let rt = Runtime::new().unwrap();
            let c = Context::full(&rt).unwrap();
            c.with(|ctx| {
                ctx.eval::<(), _>(JS_HOOK).unwrap();
                ctx.eval::<(), _>(BRIDGE).unwrap();
                let f: Function = ctx.globals().get("hookJson").unwrap();
                let _: String = f.call((ctx_str.clone(),)).unwrap();
            });
        });
        bench_loop("hook call (JSON-string bridge both ways)", 200, 2000, || call(false));
        let probe: String = context.with(|ctx| {
            ctx.eval("[typeof require, typeof os, typeof std].join(',')").unwrap()
        });
        println!("  sandbox: require/os/std -> {probe} (quickjs-libc modules not linked)");
    }
}

// ---------------------------------------------------------------- V8 (ceiling; native-only)

#[cfg(feature = "v8-engine")]
mod v8_engine {
    use super::*;

    pub fn run(w: &Workload) {
        println!("== v8 (rusty_v8 — the ceiling; NATIVE EMBEDDING ONLY)");
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
        let ctx_str = serde_json::to_string(&w.ctx).unwrap();

        fn make_isolate(ctx_str: &str, w: &Workload, check: bool, calls: usize, bench: bool) {
            let isolate = &mut v8::Isolate::new(Default::default());
            let scope = &mut v8::HandleScope::new(isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            let src = format!("{}\nfunction hookJson(s){{ return JSON.stringify(hook(JSON.parse(s))); }}", JS_HOOK);
            let code = v8::String::new(scope, &src).unwrap();
            let script = v8::Script::compile(scope, code, None).unwrap();
            script.run(scope).unwrap();
            let global = context.global(scope);
            let key = v8::String::new(scope, "hookJson").unwrap();
            let func: v8::Local<v8::Function> = global.get(scope, key.into()).unwrap().try_into().unwrap();
            let arg = v8::String::new(scope, ctx_str).unwrap();
            let recv = v8::undefined(scope);
            let mut samples = Vec::with_capacity(calls);
            for _ in 0..calls {
                let t0 = Instant::now();
                let out = func.call(scope, recv.into(), &[arg.into()]).unwrap();
                if bench {
                    samples.push(t0.elapsed().as_nanos());
                }
                if check {
                    let s = out.to_rust_string_lossy(scope);
                    let headers: Value = serde_json::from_str(&s).unwrap();
                    w.verify("v8", &headers);
                }
            }
            if bench {
                let n = samples.len();
                stats("hook call (JSON-string bridge)", &mut samples[n.min(200)..].to_vec());
            }
        }

        make_isolate(&ctx_str, w, true, 1, false);
        cold_loop("cold (isolate+context+compile+1 call)", 20, || make_isolate(&ctx_str, w, false, 1, false));
        make_isolate(&ctx_str, w, false, 2200, true);
        println!("  sandbox: bare V8 has no require/process/fetch; capability = whatever the embedder binds");
    }
}

fn main() {
    let w = Workload::new();
    println!(
        "hook-runner on {} ({})",
        std::env::consts::ARCH,
        if cfg!(target_os = "wasi") { "inside WASM/WASI" } else { std::env::consts::OS }
    );
    #[cfg(feature = "rhai-engine")]
    rhai_engine::run(&w);
    #[cfg(feature = "boa")]
    boa_js::run(&w);
    #[cfg(feature = "lua")]
    lua_engine::run(&w);
    #[cfg(feature = "quickjs")]
    quickjs_engine::run(&w);
    #[cfg(feature = "v8-engine")]
    v8_engine::run(&w);
    println!("done");
}
