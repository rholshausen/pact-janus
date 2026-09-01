use rquickjs::{Context, Runtime};
use std::time::{Duration, Instant};

const SCRIPT: &str = r#"
function hook(ctx) {
  let h = ctx.request.headers || {};
  h["x-signed-at"] = "2026-01-01T00:00:00Z";
  let s = ctx.request.method + ctx.request.path + (ctx.request.body || "") + ctx.config.secret;
  let acc = 2166136261;
  for (let i = 0; i < s.length; i++) { acc ^= s.charCodeAt(i); acc = Math.imul(acc, 16777619) >>> 0; }
  h["authorization"] = "Janus " + acc.toString(16).padStart(8, "0");
  return JSON.stringify({ outcome: "ok", changes: { "request.headers": h } });
}
"#;

const INPUT: &str = r#"{"request":{"method":"POST","path":"/orders","body":"{}","headers":{"accept":"application/json"}},"config":{"secret":"s3cr3t"}}"#;

fn main() {
    // 1. correctness: the 1.6 workload, unchanged
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    let (out, warm) = ctx.with(|c| {
        c.eval::<(), _>(SCRIPT).unwrap();
        let f: rquickjs::Function = c.globals().get("hook").unwrap();
        let parse: rquickjs::Function = c.eval("JSON.parse").unwrap();
        let call = |c: &rquickjs::Ctx| -> String {
            let arg: rquickjs::Value = parse.call((INPUT,)).unwrap();
            let _ = c;
            f.call((arg,)).unwrap()
        };
        let out = call(&c);
        let n = 2000;
        let t = Instant::now();
        for _ in 0..n { let _ = call(&c); }
        (out, t.elapsed() / n)
    });
    println!("output: {out}");
    println!("warm call: {:.1} us", warm.as_secs_f64() * 1e6);

    // 2. runaway script: interrupt handler must stop it
    let rt2 = Runtime::new().unwrap();
    let deadline = Instant::now() + Duration::from_millis(50);
    rt2.set_interrupt_handler(Some(Box::new(move || Instant::now() > deadline)));
    let ctx2 = Context::full(&rt2).unwrap();
    let t = Instant::now();
    let res: Result<(), _> = ctx2.with(|c| c.eval::<(), _>("while (true) {}"));
    println!(
        "runaway: {} after {} ms",
        match res { Ok(()) => "COMPLETED (bad)".to_string(), Err(e) => format!("interrupted ({e})") },
        t.elapsed().as_millis()
    );
}
