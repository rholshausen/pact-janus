//! The scripted-hook runtime (lifecycle-hooks spec §9, [ADR 0015](../../../../Documentation/decisions/0015-quickjs-as-the-scripted-hook-runtime.md)):
//! **QuickJS, embedded via `rquickjs`, compiled with the engine.**
//!
//! It is the only implementation available in *every* embedding, and that is the whole reason it
//! was chosen over faster options: spike 1.6 measured V8 at 6.7× QuickJS per call and rejected it
//! because it is native-only, which would have forked scripted hooks into a
//! "subprocess-embedding only" feature. QuickJS is JS (what users ask for), builds for the WASI
//! targets the primary embedding needs, is the smallest JS artifact of the candidates, and exposes
//! no ambient capabilities by default.
//!
//! **There is no `require`, no `fetch`, no file system, no environment and no timers** — not as a
//! policy this module enforces but as a fact about how a bare interpreter is built (spike 1.6
//! finding 6). Everything a hook needs from outside arrives in `ctx.config`, which the loader
//! filled in. The one thing bound from Rust is `janus.log`; the rest of the `janus` object is
//! JavaScript, evaluated beside the hook, because a helper that could have been written in the
//! language it serves should be.
//!
//! **Every invocation is bounded** (§9.5). An interrupt handler tripped by the deadline ends a
//! runaway script as `timed-out`; no interpreter in the bake-off interrupts one by default, and a
//! hook that spins would otherwise hang a verification.
//!
//! **A runtime per invocation.** Caching one per source would save the 0.17 ms cold start the spike
//! measured, and would mean one hook's leftover globals reaching the next invocation — state no
//! author declared and no reader would look for. At a handful of calls per exchange the trade is
//! not close; if a run ever has thousands, the answer is the same one `exec` has for the same
//! problem, a long-lived component.

use super::invoke::{HookFailure, HookInvoker, InvokeResult, Outcome};
use rquickjs::{Context, Function, Runtime};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// The JavaScript half of the `janus` standard library (spec §9.3, `hook-api.d.ts`). Evaluated
/// before the hook's own source, so a script sees it as an ordinary global.
const JANUS_LIB: &str = r#"
globalThis.janus = {
  text(slot) {
    if (slot === undefined || slot === null) return undefined;
    const content = slot.content;
    if (slot.encoded === "base64") return __janus_from_base64(content);
    return typeof content === "string" ? content : JSON.stringify(content);
  },
  json(slot) {
    if (slot === undefined || slot === null) return undefined;
    const content = slot.content;
    if (slot.encoded === "base64") {
      try { return JSON.parse(__janus_from_base64(content)); } catch (e) { return undefined; }
    }
    if (typeof content === "string" && slot.encoded === "json") {
      try { return JSON.parse(content); } catch (e) { return content; }
    }
    return content;
  },
  bytes(slot) {
    if (slot === undefined || slot === null) return undefined;
    if (slot.encoded === "base64") return slot.content;
    return __janus_to_base64(typeof slot.content === "string" ? slot.content : JSON.stringify(slot.content));
  },
  slot(value, options) {
    const slot = { content: value };
    if (options && options.encoded) slot.encoded = options.encoded;
    if (options && options.contentType) slot["content-type"] = options.contentType;
    return slot;
  },
  log(level, message) { __janus_log(String(level), String(message)); },
};
"#;

/// The wrapper that turns "call a function by name with a document" into one string-in/string-out
/// call, so no value converter has to exist on either side: `JSON` is already in both languages and
/// is the one they agree about.
fn invoke_wrapper(entry: &str) -> String {
  format!(
    r#"
globalThis.__janus_invoke = function (json) {{
  if (typeof {entry} !== "function") {{
    throw new Error("hook entry '{entry}' is not a function");
  }}
  const result = {entry}(JSON.parse(json));
  return result === undefined || result === null ? "" : JSON.stringify(result);
}};
"#
  )
}

#[derive(Debug, Default)]
pub struct ScriptHooks;

impl ScriptHooks {
  pub fn new() -> Self {
    ScriptHooks
  }
}

impl HookInvoker for ScriptHooks {
  fn invoke(&self, run: &Value, context: &Value, deadline_ms: u64) -> Result<InvokeResult, HookFailure> {
    let Some(source) = run.get("source").and_then(Value::as_str) else {
      // A `path` here would mean the loader did not do its job (ADR 0014): the engine reads no
      // files, so a script that arrived as a path is a configuration that never got resolved.
      return Err(HookFailure::errored(
        "a script hook needs its source inline; the loader inlines it from 'path'",
      ));
    };
    let entry = run.get("entry").and_then(Value::as_str).unwrap_or("hook");
    let document = serde_json::to_string(context).expect("a HookContext always serializes");

    let runtime = Runtime::new().map_err(|err| HookFailure::errored(format!("quickjs: {err}")))?;
    let deadline = Instant::now() + Duration::from_millis(deadline_ms);
    // §9.5: tripped by the deadline, checked by the interpreter between operations — the only
    // thing that stops `while (true) {}`.
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));

    let ctx = Context::full(&runtime).map_err(|err| HookFailure::errored(format!("quickjs: {err}")))?;
    let answered: Result<String, ScriptError> = ctx.with(|ctx| {
      bind_natives(&ctx)?;
      ctx
        .eval::<(), _>(JANUS_LIB)
        .map_err(|err| ScriptError::from(&ctx, err, "the janus library"))?;
      ctx
        .eval::<(), _>(source)
        .map_err(|err| ScriptError::from(&ctx, err, "the hook source"))?;
      ctx
        .eval::<(), _>(invoke_wrapper(entry).as_str())
        .map_err(|err| ScriptError::from(&ctx, err, "the hook wrapper"))?;

      let invoke: Function = ctx
        .globals()
        .get("__janus_invoke")
        .map_err(|err| ScriptError::from(&ctx, err, "the hook wrapper"))?;
      invoke
        .call::<_, String>((document.as_str(),))
        .map_err(|err| ScriptError::from(&ctx, err, "the hook"))
    });

    let answered = match answered {
      Ok(answered) => answered,
      Err(err) => {
        // An interrupted script and a thrown exception are different facts (§5.1): one says the
        // hook is slow, the other says it is broken, and they send a reader to different places.
        if Instant::now() >= deadline {
          return Err(HookFailure::timed_out(deadline_ms));
        }
        return Err(HookFailure {
          outcome: Outcome::Errored,
          error: json!({ "code": "hook-script-threw", "message": err.message, "details": { "at": err.at } }),
        });
      }
    };

    if answered.is_empty() {
      // Returning nothing is `{ "outcome": "ok" }` with no changes — the common case for an
      // observing hook (§9.2).
      return Ok(InvokeResult::ok());
    }
    let document: Value = serde_json::from_str(&answered).map_err(|err| HookFailure {
      outcome: Outcome::Errored,
      error: json!({ "code": "hook-result-invalid", "message": err.to_string() }),
    })?;
    InvokeResult::parse(&document).map_err(|message| HookFailure {
      outcome: Outcome::Errored,
      error: json!({ "code": "hook-result-invalid", "message": message }),
    })
  }
}

/// What a script did wrong, with the phase it happened in — "your hook threw" and "your source did
/// not even evaluate" are different messages to receive at 2am.
struct ScriptError {
  message: String,
  at: String,
}

impl ScriptError {
  fn from(ctx: &rquickjs::Ctx<'_>, err: rquickjs::Error, at: &str) -> ScriptError {
    let message = match err {
      rquickjs::Error::Exception => {
        let exception = ctx.catch();
        exception
          .as_exception()
          .and_then(|exception| exception.message())
          .unwrap_or_else(|| format!("{:?}", exception))
      }
      other => other.to_string(),
    };
    ScriptError {
      message,
      at: at.to_string(),
    }
  }
}

/// The two things JavaScript cannot do for itself here — base64, which QuickJS has no built-in for,
/// and logging, which is the only output a script has (§9.3).
fn bind_natives<'js>(ctx: &rquickjs::Ctx<'js>) -> Result<(), ScriptError> {
  let globals = ctx.globals();
  macro_rules! bind {
    ($name:expr, $function:expr) => {
      globals
        .set($name, $function)
        .map_err(|err| ScriptError::from(ctx, err, "binding the janus library"))?
    };
  }

  let log = Function::new(ctx.clone(), |level: String, message: String| {
    // A hook that logs a secret has logged it (§7.3): `janus.log` writes what it is given, and the
    // engine does not pretend it can fix that.
    match level.as_str() {
      "error" => tracing::error!(target: "janus::hook::script", "{message}"),
      "warn" => tracing::warn!(target: "janus::hook::script", "{message}"),
      "debug" => tracing::debug!(target: "janus::hook::script", "{message}"),
      _ => tracing::info!(target: "janus::hook::script", "{message}"),
    }
  })
  .map_err(|err| ScriptError::from(ctx, err, "binding janus.log"))?;
  bind!("__janus_log", log);

  let to_base64 = Function::new(ctx.clone(), |text: String| {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(text.as_bytes())
  })
  .map_err(|err| ScriptError::from(ctx, err, "binding base64"))?;
  bind!("__janus_to_base64", to_base64);

  let from_base64 = Function::new(ctx.clone(), |encoded: String| {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
      .decode(encoded.as_bytes())
      .ok()
      .and_then(|bytes| String::from_utf8(bytes).ok())
      .unwrap_or_default()
  })
  .map_err(|err| ScriptError::from(ctx, err, "binding base64"))?;
  bind!("__janus_from_base64", from_base64);

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn run(source: &str, context: Value) -> Result<InvokeResult, HookFailure> {
    ScriptHooks::new().invoke(&json!({ "kind": "script", "source": source }), &context, 5_000)
  }

  fn request_context() -> Value {
    json!({
      "point": "before-request",
      "role": "provider",
      "parts": { "request": {
        "method": { "content": "GET" },
        "path": { "content": "/orders/66" },
        "headers": { "content": { "accept": ["application/json"] } }
      } },
      "config": { "secret": "s3cr3t" },
      "mutable": ["parts.request.headers"],
      "deadline-ms": 5000
    })
  }

  #[test]
  fn a_hook_sees_the_context_and_answers_with_a_change() {
    let result = run(
      r#"
      function hook(ctx) {
        const headers = janus.json(ctx.parts.request.headers) || {};
        headers.authorization = ["Janus " + ctx.config.secret + ":" + ctx.parts.request.path.content];
        return { outcome: "ok", changes: { "parts.request.headers": janus.slot(headers) } };
      }
      "#,
      request_context(),
    )
    .expect("the script answers");
    assert_eq!(result.outcome(), Outcome::Ok);
    assert_eq!(
      result.changes["parts.request.headers"]["content"]["authorization"],
      json!(["Janus s3cr3t:/orders/66"])
    );
    assert_eq!(
      result.changes["parts.request.headers"]["content"]["accept"],
      json!(["application/json"]),
      "the script read what was there and added to it"
    );
  }

  #[test]
  fn returning_nothing_is_ok_with_no_changes() {
    let result = run("function hook(ctx) { }", request_context()).expect("answers");
    assert_eq!(result, InvokeResult::ok());
  }

  #[test]
  fn one_script_may_serve_several_points_by_branching_on_the_point() {
    let source = r#"
      function hook(ctx) {
        if (ctx.point === "state-setup") return { outcome: "unsupported",
          error: { code: "no-fixtures", message: "this script does not do state" } };
        return { outcome: "ok" };
      }
    "#;
    let setup = run(source, json!({ "point": "state-setup", "role": "provider" })).expect("answers");
    assert_eq!(setup.outcome(), Outcome::Unsupported);
    let request = run(source, request_context()).expect("answers");
    assert_eq!(request.outcome(), Outcome::Ok);
  }

  #[test]
  fn a_named_entry_function_is_called_instead_of_hook() {
    let result = ScriptHooks::new()
      .invoke(
        &json!({
          "kind": "script",
          "entry": "sign",
          "source": "function sign(ctx) { return { outcome: 'ok', data: { signed: true } }; } \
                     function hook(ctx) { return { outcome: 'failed' }; }"
        }),
        &request_context(),
        5_000,
      )
      .expect("answers");
    assert_eq!(result.outcome(), Outcome::Ok);
    assert_eq!(result.data, Some(json!({ "signed": true })));
  }

  #[test]
  fn a_missing_entry_function_is_an_error_naming_it() {
    let failure = ScriptHooks::new()
      .invoke(
        &json!({ "kind": "script", "entry": "sign", "source": "function hook(ctx) {}" }),
        &request_context(),
        5_000,
      )
      .expect_err("there is no 'sign'");
    assert_eq!(failure.outcome, Outcome::Errored);
    assert!(
      failure.error["message"].as_str().unwrap().contains("sign"),
      "{:?}",
      failure.error
    );
  }

  #[test]
  fn a_script_that_throws_is_errored_with_its_own_message() {
    let failure = run(
      "function hook(ctx) { throw new Error('the signature key is malformed'); }",
      request_context(),
    )
    .expect_err("it threw");
    assert_eq!(failure.outcome, Outcome::Errored);
    assert_eq!(failure.error["code"], json!("hook-script-threw"));
    assert!(
      failure.error["message"]
        .as_str()
        .unwrap()
        .contains("the signature key is malformed"),
      "{:?}",
      failure.error
    );
  }

  #[test]
  fn a_runaway_script_is_interrupted_and_reported_as_timed_out() {
    let started = Instant::now();
    let failure = ScriptHooks::new()
      .invoke(
        &json!({ "kind": "script", "source": "function hook(ctx) { while (true) {} }" }),
        &request_context(),
        50,
      )
      .expect_err("it never returns");
    assert_eq!(failure.outcome, Outcome::TimedOut);
    assert!(
      started.elapsed() < Duration::from_secs(5),
      "the interrupt handler stopped it, not a test timeout: {:?}",
      started.elapsed()
    );
  }

  #[test]
  fn the_interpreter_has_no_ambient_capabilities() {
    // Not a policy this module enforces — a fact about a bare interpreter (spike 1.6 finding 6).
    for forbidden in ["require", "fetch", "process", "setTimeout", "XMLHttpRequest"] {
      let source =
        format!("function hook(ctx) {{ return {{ outcome: 'ok', data: {{ has: typeof {forbidden} }} }}; }}");
      let result = run(&source, request_context()).expect("answers");
      assert_eq!(
        result.data,
        Some(json!({ "has": "undefined" })),
        "'{forbidden}' must not exist in a hook's world"
      );
    }
  }

  #[test]
  fn the_language_and_its_standard_built_ins_are_there() {
    let result = run(
      "function hook(ctx) { return { outcome: 'ok', data: {
         json: typeof JSON.stringify({}), math: Math.max(1, 2),
         date: typeof new Date().toISOString(), array: [3,1,2].sort().join(''),
       } }; }",
      request_context(),
    )
    .expect("answers");
    assert_eq!(
      result.data,
      Some(json!({ "json": "string", "math": 2, "date": "string", "array": "123" }))
    );
  }

  #[test]
  fn janus_text_and_bytes_read_an_encoded_slot() {
    let context = json!({
      "point": "after-response",
      "role": "provider",
      "parts": { "response": { "body": { "content": "eyJpZCI6Im8tMSJ9", "encoded": "base64",
                                         "content-type": "application/json" } } }
    });
    let result = run(
      r#"
      function hook(ctx) {
        const body = ctx.parts.response.body;
        return { outcome: "ok", data: {
          text: janus.text(body), id: janus.json(body).id, bytes: janus.bytes(body) } };
      }
      "#,
      context,
    )
    .expect("answers");
    assert_eq!(
      result.data,
      Some(json!({
        "text": "{\"id\":\"o-1\"}",
        "id": "o-1",
        "bytes": "eyJpZCI6Im8tMSJ9"
      }))
    );
  }

  #[test]
  fn a_script_that_arrived_as_a_path_is_a_configuration_that_was_never_resolved() {
    let failure = ScriptHooks::new()
      .invoke(
        &json!({ "kind": "script", "path": "./sign.js" }),
        &request_context(),
        5_000,
      )
      .expect_err("the engine reads no files");
    assert!(failure.error["message"].as_str().unwrap().contains("loader"));
  }

  #[test]
  fn an_unknown_outcome_from_a_script_is_refused_rather_than_guessed_at() {
    let failure = run(
      "function hook(ctx) { return { outcome: 'probably-fine' }; }",
      request_context(),
    )
    .expect_err("open vocabularies are not 'anything'");
    assert_eq!(failure.error["code"], json!("hook-result-invalid"));
  }
}
