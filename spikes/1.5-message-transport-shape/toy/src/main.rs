//! Spike 1.5 toy: one role-neutral transport interface, implemented by a real
//! HTTP transport and an in-memory broker transport, driven through the same
//! engine-side loop across five scenarios (see README).
//!
//! Deliberate shortcuts (this is a shape test, not an implementation): bodies
//! travel as parsed JSON parts (a content component would own bytes<->doc);
//! HTTP parsing is minimal; the broker is in-process.

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------- interface

/// Arrived wire traffic, mapped to abstract parts.
struct InboundEvent {
    id: u64,
    parts: Value,
}

/// The four role-neutral primitives distilled from the RFC's transport
/// sketch ("start/stop a mock endpoint; drive requests at a provider; map
/// wire messages to/from the abstract interaction parts").
trait Transport {
    /// Bring the wire up. Returns an open endpoint-descriptor document.
    fn start(&mut self, options: &Value) -> Value;
    fn stop(&mut self);
    /// Outbound: put parts on the wire. Optionally await a correlated reply.
    fn send(&mut self, parts: &Value, await_reply: bool) -> Result<Option<Value>, String>;
    /// Inbound: next arrived event, if any, mapped to parts.
    fn poll_inbound(&mut self, timeout: Duration) -> Option<InboundEvent>;
    /// Complete an inbound event that requires a wire reply.
    fn reply(&mut self, event_id: u64, parts: &Value) -> Result<(), String>;
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

// ------------------------------------------------------------ HTTP transport

struct HttpTransport {
    port: u16,
    rx: Option<Receiver<InboundEvent>>,
    pending: Arc<Mutex<HashMap<u64, TcpStream>>>,
    shutdown: Arc<AtomicBool>,
}

impl HttpTransport {
    fn new() -> Self {
        Self { port: 0, rx: None, pending: Arc::default(), shutdown: Arc::default() }
    }
}

fn read_http(stream: &mut TcpStream) -> Result<(String, HashMap<String, String>, Value), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut start_line = String::new();
    reader.read_line(&mut start_line).map_err(|e| e.to_string())?;
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let body = if len > 0 {
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
        serde_json::from_slice(&buf).unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    Ok((start_line.trim_end().to_string(), headers, body))
}

fn write_http_response(stream: &mut TcpStream, status: u64, body: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(body).map_err(|e| e.to_string())?;
    write!(stream, "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n", bytes.len())
        .and_then(|_| stream.write_all(&bytes))
        .map_err(|e| e.to_string())
}

impl Transport for HttpTransport {
    fn start(&mut self, _options: &Value) -> Value {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        self.port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let pending = self.pending.clone();
        let shutdown = self.shutdown.clone();
        thread::spawn(move || {
            for conn in listener.incoming() {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(mut stream) = conn else { break };
                let Ok((start_line, headers, body)) = read_http(&mut stream) else { continue };
                let mut it = start_line.split_whitespace();
                let (method, path) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
                // wire -> abstract parts
                let parts = json!({
                    "method": method, "path": path,
                    "headers": headers, "body": body,
                });
                let id = next_id();
                pending.lock().unwrap().insert(id, stream);
                if tx.send(InboundEvent { id, parts }).is_err() {
                    break;
                }
            }
        });
        json!({ "transport": "http", "scheme": "http", "host": "127.0.0.1", "port": self.port })
    }

    fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(("127.0.0.1", self.port)); // unblock accept
    }

    fn send(&mut self, parts: &Value, await_reply: bool) -> Result<Option<Value>, String> {
        // abstract parts -> wire request
        let port = parts["port"].as_u64().ok_or("send needs port")? as u16;
        let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
        let body = serde_json::to_vec(&parts["body"]).map_err(|e| e.to_string())?;
        write!(
            stream,
            "{} {} HTTP/1.1\r\nhost: x\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
            parts["method"].as_str().unwrap_or("GET"),
            parts["path"].as_str().unwrap_or("/"),
            body.len()
        )
        .and_then(|_| stream.write_all(&body))
        .map_err(|e| e.to_string())?;
        if !await_reply {
            return Ok(None); // request/response transport degenerates fine
        }
        // wire response -> abstract parts
        let (start_line, headers, body) = read_http(&mut stream)?;
        let status: u64 = start_line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        Ok(Some(json!({ "status": status, "headers": headers, "body": body })))
    }

    fn poll_inbound(&mut self, timeout: Duration) -> Option<InboundEvent> {
        self.rx.as_ref()?.recv_timeout(timeout).ok()
    }

    fn reply(&mut self, event_id: u64, parts: &Value) -> Result<(), String> {
        let mut stream =
            self.pending.lock().unwrap().remove(&event_id).ok_or("unknown or already-replied event")?;
        write_http_response(&mut stream, parts["status"].as_u64().unwrap_or(200), &parts["body"])
    }
}

// ------------------------------------------- in-memory broker ("external" system)

#[derive(Clone, Default)]
struct Broker {
    topics: Arc<Mutex<HashMap<String, Vec<Sender<Value>>>>>,
}

impl Broker {
    fn subscribe(&self, topic: &str) -> Receiver<Value> {
        let (tx, rx) = mpsc::channel();
        self.topics.lock().unwrap().entry(topic.to_string()).or_default().push(tx);
        rx
    }
    fn publish(&self, message: Value) {
        let topic = message["topic"].as_str().unwrap_or("").to_string();
        if let Some(subs) = self.topics.lock().unwrap().get(&topic) {
            for tx in subs {
                let _ = tx.send(message.clone());
            }
        }
    }
}

// ----------------------------------------------------------- broker transport

struct BrokerTransport {
    broker: Broker,
    subs: Vec<Receiver<Value>>,
    /// inbound events whose reply-routing info we must remember
    pending: HashMap<u64, Value>,
}

impl BrokerTransport {
    fn new(broker: Broker) -> Self {
        Self { broker, subs: Vec::new(), pending: HashMap::new() }
    }
}

impl Transport for BrokerTransport {
    fn start(&mut self, options: &Value) -> Value {
        let topics: Vec<String> = options["subscribe"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        for t in &topics {
            self.subs.push(self.broker.subscribe(t));
        }
        json!({ "transport": "message", "broker": "inmem", "subscribed": topics })
    }

    fn stop(&mut self) {
        self.subs.clear();
    }

    fn send(&mut self, parts: &Value, await_reply: bool) -> Result<Option<Value>, String> {
        if !await_reply {
            self.broker.publish(parts.clone()); // fire-and-forget
            return Ok(None);
        }
        // request/reply over the broker: reply-topic + correlation-id
        let reply_topic = format!("replies.{}", next_id());
        let correlation = format!("corr-{}", next_id());
        let reply_rx = self.broker.subscribe(&reply_topic);
        let mut msg = parts.clone();
        msg["headers"]["reply-topic"] = json!(reply_topic);
        msg["headers"]["correlation-id"] = json!(correlation);
        self.broker.publish(msg);
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let remaining = deadline.checked_duration_since(Instant::now()).ok_or("reply timeout")?;
            let candidate = reply_rx.recv_timeout(remaining).map_err(|_| "reply timeout")?;
            if candidate["headers"]["correlation-id"] == json!(correlation) {
                return Ok(Some(candidate));
            }
        }
    }

    fn poll_inbound(&mut self, timeout: Duration) -> Option<InboundEvent> {
        let deadline = Instant::now() + timeout;
        loop {
            for rx in &self.subs {
                if let Ok(msg) = rx.try_recv() {
                    let id = next_id();
                    self.pending.insert(id, msg.clone());
                    return Some(InboundEvent { id, parts: msg });
                }
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn reply(&mut self, event_id: u64, parts: &Value) -> Result<(), String> {
        let inbound = self.pending.remove(&event_id).ok_or("unknown event")?;
        let reply_topic = inbound["headers"]["reply-topic"].as_str().ok_or("inbound message declared no reply-topic")?;
        let mut msg = parts.clone();
        msg["topic"] = json!(reply_topic);
        msg["headers"]["correlation-id"] = inbound["headers"]["correlation-id"].clone();
        self.broker.publish(msg);
        Ok(())
    }
}

// ------------------------------------------------- engine-side toy matching

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
            if let Some(t) = exp.first() {
                for (i, a) in act.iter().enumerate() {
                    match_type(t, a, &format!("{path}[{i}]"), mismatches);
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

/// Match selected parts of an interaction: exact keys (metadata identity like
/// `topic`) plus type-shaped keys — a stand-in for real plan execution.
fn match_parts(exact: &[(&str, &Value)], shaped: &[(&str, &Value)], actual: &Value) -> Vec<Value> {
    let mut mismatches = Vec::new();
    for (key, expected) in exact {
        if actual.get(key) != Some(expected) {
            mismatches.push(json!({ "path": format!("$.{key}"), "expected": expected, "actual": actual.get(key) }));
        }
    }
    for (key, expected) in shaped {
        match actual.get(key) {
            Some(act) => match_type(expected, act, &format!("$.{key}"), &mut mismatches),
            None => mismatches.push(json!({ "path": format!("$.{key}"), "expected": "present", "actual": "missing" })),
        }
    }
    mismatches
}

fn ok(cond: bool, msg: &str) {
    assert!(cond, "FAIL: {msg}");
    println!("  ok: {msg}");
}

// ------------------------------------------------------------------ scenarios

fn main() {
    let timeout = Duration::from_secs(2);

    println!("S1: consumer HTTP mock (passive: app initiates, engine replies)");
    {
        let mut transport = HttpTransport::new();
        let endpoint = transport.start(&json!({}));
        println!("  endpoint descriptor: {endpoint}");
        let port = endpoint["port"].as_u64().unwrap() as u16;
        // the consumer app under test, in a thread, speaking real HTTP
        let app = thread::spawn(move || {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            let body = br#"{"sku":"widget-1","quantity":2}"#;
            write!(stream, "POST /orders HTTP/1.1\r\nhost: x\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n", body.len()).unwrap();
            stream.write_all(body).unwrap();
            let (start_line, _h, resp_body) = read_http(&mut stream).unwrap();
            (start_line, resp_body)
        });
        let event = transport.poll_inbound(timeout).expect("request arrives");
        let mismatches = match_parts(
            &[("method", &json!("POST")), ("path", &json!("/orders"))],
            &[("body", &json!({ "sku": "s", "quantity": 1 }))],
            &event.parts,
        );
        ok(mismatches.is_empty(), &format!("arrived request matched (mismatches: {mismatches:?})"));
        transport.reply(event.id, &json!({ "status": 201, "body": { "id": "ORD-1", "status": "PENDING" } })).unwrap();
        let (status_line, resp_body) = app.join().unwrap();
        ok(status_line.contains("201") && resp_body["id"] == json!("ORD-1"), "app saw the mocked response");
        transport.stop();
    }

    println!("S2: provider HTTP verify (active: engine drives request)");
    {
        // the provider, in a thread: one real HTTP endpoint
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_http(&mut stream).unwrap();
            write_http_response(&mut stream, 200, &json!({ "id": "ORD-9", "status": "SHIPPED" })).unwrap();
        });
        let mut transport = HttpTransport::new();
        let response = transport
            .send(&json!({ "port": port, "method": "GET", "path": "/orders/ORD-9", "body": null }), true)
            .unwrap()
            .expect("response parts");
        let mismatches = match_parts(
            &[("status", &json!(200))],
            &[("body", &json!({ "id": "s", "status": "s" }))],
            &response,
        );
        ok(mismatches.is_empty(), &format!("provider response matched (mismatches: {mismatches:?})"));
    }

    let broker = Broker::default();

    println!("S3: consumer message test (engine emits; 'serve-variant' = 'publish now')");
    {
        // the consumer app's subscription + handler, in a thread
        let app_rx = broker.subscribe("orders.created");
        let handler = thread::spawn(move || {
            let msg = app_rx.recv_timeout(Duration::from_secs(2)).expect("app receives event");
            // the app's handler logic: reads fields it depends on
            (msg["payload"]["id"].as_str().map(String::from), msg["headers"]["content-type"].clone())
        });
        let mut transport = BrokerTransport::new(broker.clone());
        let endpoint = transport.start(&json!({}));
        println!("  endpoint descriptor: {endpoint}");
        transport
            .send(
                &json!({
                    "topic": "orders.created", "key": "ORD-1",
                    "headers": { "content-type": "application/json" },
                    "payload": { "id": "ORD-1", "status": "PENDING", "amount": 12.5 },
                }),
                false, // fire-and-forget: no reply half exists
            )
            .unwrap();
        let (seen_id, ct) = handler.join().unwrap();
        ok(seen_id.as_deref() == Some("ORD-1") && ct == json!("application/json"), "app handler consumed the emitted example message");
        transport.stop();
    }

    println!("S4: provider message verify (unsolicited inbound, fire-and-forget, metadata matched)");
    {
        let mut transport = BrokerTransport::new(broker.clone());
        transport.start(&json!({ "subscribe": ["orders.created"] }));
        // the provider app's producer, triggered like a produce-message hook
        let b = broker.clone();
        let trigger_produce = move || {
            b.publish(json!({
                "topic": "orders.created", "key": "ORD-77",
                "headers": { "content-type": "application/json" },
                "payload": { "id": "ORD-77", "status": "SHIPPED", "amount": 99.0 },
            }));
        };
        trigger_produce();
        let event = transport.poll_inbound(timeout).expect("message collected");
        let mismatches = match_parts(
            &[("topic", &json!("orders.created"))],
            &[("key", &json!("k")), ("headers", &json!({ "content-type": "ct" })), ("payload", &json!({ "id": "s", "status": "s", "amount": 1.0 }))],
            &event.parts,
        );
        ok(mismatches.is_empty(), &format!("produced message matched incl. metadata (mismatches: {mismatches:?})"));
        // fire-and-forget: nothing to reply to — dropping the event is the correct end state
        transport.stop();
    }

    println!("S5a: sync message, provider verify (engine sends, awaits correlated reply)");
    {
        // the provider's responder, in a thread
        let resp_rx = broker.subscribe("orders.query");
        let b = broker.clone();
        thread::spawn(move || {
            let req = resp_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            b.publish(json!({
                "topic": req["headers"]["reply-topic"],
                "headers": { "correlation-id": req["headers"]["correlation-id"], "content-type": "application/json" },
                "payload": { "id": req["payload"]["id"], "status": "PENDING" },
            }));
        });
        let mut transport = BrokerTransport::new(broker.clone());
        transport.start(&json!({}));
        let reply = transport
            .send(&json!({ "topic": "orders.query", "headers": {}, "payload": { "id": "ORD-5" } }), true)
            .unwrap()
            .expect("correlated reply");
        let mismatches = match_parts(&[], &[("payload", &json!({ "id": "s", "status": "s" }))], &reply);
        ok(mismatches.is_empty(), &format!("correlated reply matched (mismatches: {mismatches:?})"));
        transport.stop();
    }

    println!("S5b: sync message, consumer mock (engine replies — same loop as the HTTP mock)");
    {
        let mut transport = BrokerTransport::new(broker.clone());
        transport.start(&json!({ "subscribe": ["orders.query2"] }));
        // the consumer app: sends a request message, awaits the correlated reply
        let b = broker.clone();
        let app = thread::spawn(move || {
            let reply_rx = b.subscribe("app.replies");
            b.publish(json!({
                "topic": "orders.query2",
                "headers": { "reply-topic": "app.replies", "correlation-id": "corr-app-1" },
                "payload": { "id": "ORD-6" },
            }));
            reply_rx.recv_timeout(Duration::from_secs(2)).expect("app receives reply")
        });
        let event = transport.poll_inbound(timeout).expect("request message arrives");
        let mismatches = match_parts(&[("topic", &json!("orders.query2"))], &[("payload", &json!({ "id": "s" }))], &event.parts);
        ok(mismatches.is_empty(), "arrived request-message matched");
        transport.reply(event.id, &json!({ "headers": {}, "payload": { "id": "ORD-6", "status": "PENDING" } })).unwrap();
        let reply = app.join().unwrap();
        ok(
            reply["headers"]["correlation-id"] == json!("corr-app-1") && reply["payload"]["status"] == json!("PENDING"),
            "app saw the mocked, correlation-preserving reply",
        );
        transport.stop();
    }

    // keep unused-warning noise down in this toy
    let _ = Map::<String, Value>::new();
    println!("all scenarios passed");
}
