//! Benchmark baseline harness (plan task 1.7).
//!
//! Measures today's stack — pact_ffi (pinned in Cargo.toml) — at the same
//! boundary the language SDKs use: the FFI functions. Scenarios cover mock
//! server startup/latency/throughput (small and ~100 KB bodies, match and
//! mismatch paths) and verification wall-time over a generated pact corpus.
//! Results land in `results/<date>-<stack>.json`; Janus runs the same
//! scenarios from Phase 4 on, producing the trend line the RFC's
//! "performance envelope of WASM vs native FFI" question needs.

mod util;

use serde_json::{json, Value};
use std::collections::HashMap;
use std::ffi::CString;
use std::process::Command;
use std::time::Instant;

use pact_ffi::mock_server::handles::{
    pactffi_new_interaction, pactffi_new_pact, pactffi_pact_handle_write_file, pactffi_response_status,
    pactffi_with_body, pactffi_with_request, InteractionPart, PactHandle,
};
use pact_ffi::mock_server::{
    pactffi_cleanup_mock_server, pactffi_create_mock_server_for_transport, pactffi_mock_server_matched,
};
use pact_ffi::verifier::{
    pactffi_verifier_add_file_source, pactffi_verifier_execute, pactffi_verifier_new_for_application,
    pactffi_verifier_set_provider_info, pactffi_verifier_shutdown,
};

use util::{order_doc, type_matched, HttpClient, ProviderStub, RunResults, Samples};

const STACK: &str = "pact_ffi-0.5.6";

fn cs(s: &str) -> CString {
    CString::new(s).expect("no interior NULs")
}

/// Build a one-interaction pact and start a mock server for it.
/// Returns the port; caller must `pactffi_cleanup_mock_server(port)`.
fn start_mock_server(description: &str, request_body: &Value, response_body: &Value) -> i32 {
    let pact = pactffi_new_pact(cs("bench-consumer").as_ptr(), cs("bench-provider").as_ptr());
    let interaction = pactffi_new_interaction(pact, cs(description).as_ptr());
    assert!(pactffi_with_request(interaction, cs("POST").as_ptr(), cs("/orders").as_ptr()));
    assert!(pactffi_with_body(
        interaction,
        InteractionPart::Request,
        cs("application/json").as_ptr(),
        cs(&request_body.to_string()).as_ptr()
    ));
    assert!(pactffi_response_status(interaction, 201));
    assert!(pactffi_with_body(
        interaction,
        InteractionPart::Response,
        cs("application/json").as_ptr(),
        cs(&response_body.to_string()).as_ptr()
    ));
    let port = pactffi_create_mock_server_for_transport(
        pact,
        cs("127.0.0.1").as_ptr(),
        0,
        cs("http").as_ptr(),
        std::ptr::null(),
    );
    assert!(port > 0, "mock server failed to start: {port}");
    port
}

fn small_request_matchers() -> Value {
    json!({ "sku": type_matched(json!("widget-1")), "quantity": type_matched(json!(2)) })
}

fn small_response() -> Value {
    json!({ "id": "ORD-1", "status": "PENDING" })
}

// ------------------------------------------------------- mock server scenarios

fn mock_server_startup(results: &mut RunResults) {
    let samples = Samples::collect(2, 20, || {
        let port = start_mock_server("startup", &small_request_matchers(), &small_response());
        pactffi_cleanup_mock_server(port);
    });
    results.record("mock-server-startup-cycle", samples.metrics());
}

fn mock_server_latency_small(results: &mut RunResults) {
    let port = start_mock_server("small", &small_request_matchers(), &small_response());
    let mut client = HttpClient::new(port as u16);
    let body = br#"{"sku":"gadget-9","quantity":7}"#;
    let samples = Samples::collect(500, 5000, || {
        let (status, _) = client.post("/orders", body);
        assert_eq!(status, 201);
    });
    assert!(pactffi_mock_server_matched(port), "all requests should have matched");
    pactffi_cleanup_mock_server(port);
    results.record("mock-server-latency-small", samples.metrics());
}

fn mock_server_latency_100k(results: &mut RunResults) {
    let doc = order_doc(100 * 1024);
    let port = start_mock_server("large", &type_matched(doc.clone()), &small_response());
    let mut client = HttpClient::new(port as u16);
    let body = serde_json::to_vec(&doc).unwrap();
    let samples = Samples::collect(50, 500, || {
        let (status, _) = client.post("/orders", &body);
        assert_eq!(status, 201);
    });
    assert!(pactffi_mock_server_matched(port));
    pactffi_cleanup_mock_server(port);
    results.record("mock-server-latency-100k", samples.metrics());
}

fn mock_server_mismatch(results: &mut RunResults) {
    let port = start_mock_server("mismatch", &small_request_matchers(), &small_response());
    let mut client = HttpClient::new(port as u16);
    let body = br#"{"sku":123,"quantity":"two"}"#; // both fields wrong type
    let samples = Samples::collect(50, 500, || {
        let (status, _) = client.post("/orders", body);
        assert_eq!(status, 500);
    });
    assert!(!pactffi_mock_server_matched(port), "requests should have mismatched");
    pactffi_cleanup_mock_server(port);
    results.record("mock-server-latency-mismatch", samples.metrics());
}

fn mock_server_throughput(results: &mut RunResults) {
    let port = start_mock_server("throughput", &small_request_matchers(), &small_response());
    let threads = 4;
    let duration = std::time::Duration::from_secs(2);
    let total: u64 = std::thread::scope(|scope| {
        (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut client = HttpClient::new(port as u16);
                    let body = br#"{"sku":"gadget-9","quantity":7}"#;
                    let mut count = 0u64;
                    let t0 = Instant::now();
                    while t0.elapsed() < duration {
                        let (status, _) = client.post("/orders", body);
                        assert_eq!(status, 201);
                        count += 1;
                    }
                    count
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum()
    });
    pactffi_cleanup_mock_server(port);
    let rps = total as f64 / duration.as_secs_f64();
    results.record(
        "mock-server-throughput-4threads",
        json!({ "threads": threads, "seconds": duration.as_secs(), "requests": total, "rps": rps.round() }),
    );
}

// ------------------------------------------------------- verification scenarios

/// Writes a corpus of pacts through the FFI authoring API and returns the
/// pact file paths plus the provider-stub route table.
fn build_corpus(dir: &std::path::Path, pacts: usize, interactions: usize, body: &Value) -> (Vec<String>, HashMap<String, Vec<u8>>) {
    std::fs::create_dir_all(dir).unwrap();
    let mut files = Vec::new();
    let mut routes = HashMap::new();
    for p in 0..pacts {
        let consumer = format!("bench-consumer-{p}");
        let pact: PactHandle = pactffi_new_pact(cs(&consumer).as_ptr(), cs("bench-provider").as_ptr());
        for i in 0..interactions {
            let path = format!("/orders/{p}/{i}");
            let interaction = pactffi_new_interaction(pact, cs(&format!("get order {p}/{i}")).as_ptr());
            assert!(pactffi_with_request(interaction, cs("GET").as_ptr(), cs(&path).as_ptr()));
            assert!(pactffi_response_status(interaction, 200));
            assert!(pactffi_with_body(
                interaction,
                InteractionPart::Response,
                cs("application/json").as_ptr(),
                cs(&type_matched(body.clone()).to_string()).as_ptr()
            ));
            routes.insert(path, serde_json::to_vec(body).unwrap());
        }
        let rc = pactffi_pact_handle_write_file(pact, cs(dir.to_str().unwrap()).as_ptr(), true);
        assert_eq!(rc, 0, "writing pact file failed");
        files.push(dir.join(format!("{consumer}-bench-provider.json")).display().to_string());
    }
    (files, routes)
}

fn run_verifier(files: &[String], provider_port: u16) -> std::time::Duration {
    let t0 = Instant::now();
    let handle = pactffi_verifier_new_for_application(cs("pact-bench").as_ptr(), cs("0").as_ptr());
    pactffi_verifier_set_provider_info(
        handle,
        cs("bench-provider").as_ptr(),
        cs("http").as_ptr(),
        cs("127.0.0.1").as_ptr(),
        provider_port,
        cs("/").as_ptr(),
    );
    for file in files {
        pactffi_verifier_add_file_source(handle, cs(file).as_ptr());
    }
    let rc = pactffi_verifier_execute(handle);
    pactffi_verifier_shutdown(handle);
    assert_eq!(rc, 0, "verification failed");
    t0.elapsed()
}

fn verify_scenario(results: &mut RunResults, name: &str, pacts: usize, interactions: usize, body: &Value, runs: usize) {
    let dir = std::env::temp_dir().join(format!("pact-bench-{name}-{}", std::process::id()));
    let (files, routes) = build_corpus(&dir, pacts, interactions, body);
    let provider = ProviderStub::start(routes);
    let mut times: Vec<f64> = (0..runs).map(|_| run_verifier(&files, provider.port).as_secs_f64() * 1e3).collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = times[times.len() / 2];
    let total_interactions = (pacts * interactions) as f64;
    results.record(
        name,
        json!({
            "pacts": pacts,
            "interactions": pacts * interactions,
            "runs": runs,
            "median_total_ms": (median * 10.0).round() / 10.0,
            "median_ms_per_interaction": (median / total_interactions * 100.0).round() / 100.0,
        }),
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn main() {
    let date = Command::new("date")
        .arg("+%F")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown-date".to_string());

    println!("pact-bench — stack {STACK} — {date}");
    let mut results = RunResults::new(STACK);

    println!("mock server scenarios:");
    mock_server_startup(&mut results);
    mock_server_latency_small(&mut results);
    mock_server_latency_100k(&mut results);
    mock_server_mismatch(&mut results);
    mock_server_throughput(&mut results);

    println!("verification scenarios:");
    let small_order = json!({
        "id": "ORD-1", "status": "PENDING",
        "customer": { "id": 42, "name": "Customer 7", "email": "customer7@example.com" },
        "lines": [ { "line": 1, "sku": "widget-1", "quantity": 2, "price": 12.5 } ],
        "created": "2026-08-23T10:00:00Z",
        "tags": ["widget", "flange"],
    });
    verify_scenario(&mut results, "verify-corpus-small", 10, 5, &small_order, 5);
    verify_scenario(&mut results, "verify-corpus-100k", 1, 5, &order_doc(100 * 1024), 3);

    let path = results.write(&date).expect("write results");
    println!("results written to {path}");
}
