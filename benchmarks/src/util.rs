//! Support code for the benchmark harness: deterministic payloads, a
//! keep-alive HTTP client, a provider stub, timing stats, and the results
//! schema shared between stacks (pact_ffi today, Janus from Phase 4).

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

// ------------------------------------------------------------- payloads

/// Deterministic ~`target_bytes` order-history document (same shape as the
/// spike 1.2 payload, regenerated in Rust so the harness is self-contained).
pub fn order_doc(target_bytes: usize) -> Value {
    let mut seed: u64 = 42;
    let mut rand = move || {
        seed = (seed * 48271) % 2147483647;
        seed as f64 / 2147483647.0
    };
    let statuses = ["fulfilled", "pending", "shipped", "cancelled"];
    let items = ["widget", "gadget", "sprocket", "flange", "grommet"];
    let mut orders = Vec::new();
    let mut size_estimate = 16;
    while size_estimate < target_bytes {
        let lines: Vec<Value> = (0..(1 + (rand() * 4.0) as usize))
            .map(|i| {
                json!({
                    "line": i + 1,
                    "sku": format!("{}-{}", items[(rand() * items.len() as f64) as usize], (rand() * 100.0) as u64),
                    "quantity": 1 + (rand() * 9.0) as u64,
                    "price": (rand() * 10000.0).round() / 100.0,
                })
            })
            .collect();
        let order = json!({
            "id": format!("ORD-{}", orders.len() + 1),
            "status": statuses[(rand() * statuses.len() as f64) as usize],
            "customer": {
                "id": (rand() * 100000.0) as u64,
                "name": format!("Customer {}", (rand() * 1000.0) as u64),
                "email": format!("customer{}@example.com", (rand() * 1000.0) as u64),
            },
            "lines": lines,
            "created": "2026-08-23T10:00:00Z",
            "tags": [items[(rand() * items.len() as f64) as usize], items[(rand() * items.len() as f64) as usize]],
        });
        size_estimate += order.to_string().len() + 1;
        orders.push(order);
    }
    json!({ "orders": orders })
}

/// Wrap every leaf of a small body in the FFI integration-JSON type matcher.
pub fn type_matched(value: Value) -> Value {
    json!({ "value": value, "pact:matcher:type": "type" })
}

// ------------------------------------------------------------- timing

pub struct Samples(Vec<u128>);

impl Samples {
    pub fn collect(warmup: usize, iters: usize, mut f: impl FnMut()) -> Self {
        for _ in 0..warmup {
            f();
        }
        let mut samples = Vec::with_capacity(iters);
        for _ in 0..iters {
            let t0 = Instant::now();
            f();
            samples.push(t0.elapsed().as_nanos());
        }
        Self(samples)
    }

    pub fn metrics(mut self) -> Value {
        self.0.sort_unstable();
        let at = |q: f64| self.0[((q * self.0.len() as f64) as usize).min(self.0.len() - 1)] as f64 / 1e3;
        json!({
            "iterations": self.0.len(),
            "median_us": (at(0.5) * 10.0).round() / 10.0,
            "p95_us": (at(0.95) * 10.0).round() / 10.0,
            "min_us": (self.0[0] as f64 / 1e3 * 10.0).round() / 10.0,
        })
    }
}

pub fn print_scenario(name: &str, metrics: &Value) {
    println!("  {name}: {metrics}");
}

// ------------------------------------------------------- HTTP client

/// Minimal keep-alive HTTP/1.1 client; reconnects if the server closes the
/// connection. Hand-rolled so client overhead is small and constant.
pub struct HttpClient {
    port: u16,
    stream: Option<TcpStream>,
}

impl HttpClient {
    pub fn new(port: u16) -> Self {
        Self { port, stream: None }
    }

    pub fn post(&mut self, path: &str, body: &[u8]) -> (u16, Vec<u8>) {
        self.request("POST", path, body)
    }

    pub fn request(&mut self, method: &str, path: &str, body: &[u8]) -> (u16, Vec<u8>) {
        for attempt in 0..2 {
            if self.stream.is_none() {
                let stream = TcpStream::connect(("127.0.0.1", self.port)).expect("connect");
                stream.set_nodelay(true).expect("nodelay");
                self.stream = Some(stream);
            }
            match self.try_request(method, path, body) {
                Ok(result) => return result,
                Err(_) if attempt == 0 => self.stream = None, // stale keep-alive: reconnect
                Err(e) => panic!("request failed after reconnect: {e}"),
            }
        }
        unreachable!()
    }

    fn try_request(&mut self, method: &str, path: &str, body: &[u8]) -> std::io::Result<(u16, Vec<u8>)> {
        let stream = self.stream.as_mut().unwrap();
        // One write per request: split header/body writes interact with
        // Nagle + delayed ACK and inject a flat ~40 ms artifact.
        let mut message = format!(
            "{method} {path} HTTP/1.1\r\nhost: 127.0.0.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        message.extend_from_slice(body);
        stream.write_all(&message)?;
        read_http_message(stream)
    }
}

/// Reads one HTTP message (status line or request line + headers + body).
/// Returns (status-or-zero, body). Shared by client and provider stub.
pub fn read_http_message(stream: &mut TcpStream) -> std::io::Result<(u16, Vec<u8>)> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line)? == 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "closed"));
    }
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    let status = first_line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    Ok((status, body))
}

// ---------------------------------------------------- provider stub

/// Threaded keep-alive HTTP server answering GET requests from a fixed
/// path -> JSON body map. Stands in for the provider under verification.
pub struct ProviderStub {
    pub port: u16,
    shutdown: Arc<AtomicBool>,
}

impl ProviderStub {
    pub fn start(routes: HashMap<String, Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind provider stub");
        let port = listener.local_addr().unwrap().port();
        let shutdown = Arc::new(AtomicBool::new(false));
        let flag = shutdown.clone();
        let routes = Arc::new(routes);
        thread::spawn(move || {
            for conn in listener.incoming() {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(stream) = conn else { break };
                let _ = stream.set_nodelay(true);
                let routes = routes.clone();
                thread::spawn(move || serve_connection(stream, &routes));
            }
        });
        Self { port, shutdown }
    }
}

fn serve_connection(mut stream: TcpStream, routes: &HashMap<String, Vec<u8>>) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    loop {
        let mut request_line = String::new();
        match reader.read_line(&mut request_line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
        if content_length > 0 {
            let mut sink = vec![0u8; content_length];
            if reader.read_exact(&mut sink).is_err() {
                return;
            }
        }
        let path = request_line.split_whitespace().nth(1).unwrap_or("/").to_string();
        let (status, body): (u16, &[u8]) = match routes.get(&path) {
            Some(body) => (200, body),
            None => (404, b"{}"),
        };
        let mut message = format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        message.extend_from_slice(body);
        if stream.write_all(&message).is_err() {
            return;
        }
    }
}

impl Drop for ProviderStub {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

// ------------------------------------------------------------- results

/// One run = one JSON document under `results/`, schema shared across
/// stacks so Phase-4+ Janus runs produce comparable trend data.
pub struct RunResults {
    stack: String,
    scenarios: Map<String, Value>,
}

impl RunResults {
    pub fn new(stack: &str) -> Self {
        Self { stack: stack.to_string(), scenarios: Map::new() }
    }

    pub fn record(&mut self, scenario: &str, metrics: Value) {
        print_scenario(scenario, &metrics);
        self.scenarios.insert(scenario.to_string(), metrics);
    }

    pub fn write(self, date: &str) -> std::io::Result<String> {
        let doc = json!({
            "schema": 1,
            "date": date,
            "stack": self.stack,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "scenarios": Value::Object(self.scenarios),
        });
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("results");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{date}-{}.json", self.stack.replace([' ', '/'], "-")));
        std::fs::write(&path, serde_json::to_string_pretty(&doc)?)?;
        Ok(path.display().to_string())
    }
}
