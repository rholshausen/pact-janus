//! The HTTP client options (component-interfaces spec §5.6, ADR 0035), each against a real server:
//! which certificates are trusted, a client certificate a provider demands, a proxy, which HTTP
//! versions are offered and spoken, and how long a request may take.

use pact_janus_http_client::{ClientOptions, Defaults, HttpClient, HttpVersion, Proxy, SendError};
use pact_janus_test_servers::{Authority, Response, TlsProvider};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::io::Read;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

fn defaults() -> Defaults {
  Defaults {
    request_timeout: Duration::from_secs(5),
    retries: 0,
    user_agent: "pact-janus-tests".to_string(),
  }
}

fn client(options: Value) -> HttpClient {
  HttpClient::from_value(Some(&options), defaults()).expect("the options are valid")
}

fn get(url: &str) -> http::Request<Vec<u8>> {
  http::Request::get(url).body(Vec::new()).expect("a GET builds")
}

/// A TLS provider with a certificate from an authority nothing trusts, answering every request.
fn provider(authority: &Authority, clients: Option<&Authority>) -> TlsProvider {
  TlsProvider::start(
    &authority.server(&["127.0.0.1", "localhost"]),
    clients,
    |request| Response::json(200, &format!("{{\"path\":\"{}\"}}", request.target)),
  )
}

#[test]
fn a_certificate_nothing_trusts_is_refused() {
  let authority = Authority::new("Untrusted CA");
  let provider = provider(&authority, None);
  let err = client(json!({}))
    .run(get(&format!("{}/orders", provider.base_url)), None)
    .expect_err("the platform does not trust a CA made for this test");
  let SendError::Connect(message) = &err else {
    panic!("a handshake failure is a connect failure: {err}");
  };
  assert!(message.contains("certificate"), "{message}");
}

#[test]
fn insecure_accepts_any_certificate() {
  let authority = Authority::new("Untrusted CA");
  let provider = provider(&authority, None);
  let response = client(json!({ "tls": { "insecure": true } }))
    .run(get(&format!("{}/orders", provider.base_url)), None)
    .expect("an insecure client accepts the certificate");
  assert_eq!(response.status(), 200);
  assert_eq!(response.body(), br#"{"path":"/orders"}"#);
}

#[test]
fn a_ca_certificate_is_trusted_beside_the_platforms() {
  let authority = Authority::new("Private CA");
  let provider = provider(&authority, None);
  let response = client(json!({ "tls": { "ca-certificates": [authority.certificate()] } }))
    .run(get(&format!("{}/orders", provider.base_url)), None)
    .expect("the private CA is trusted");
  assert_eq!(response.status(), 200);
}

#[test]
fn a_client_certificate_is_presented_when_the_provider_demands_one() {
  let authority = Authority::new("Private CA");
  let provider = provider(&authority, Some(&authority));
  let trusting = json!({ "tls": { "ca-certificates": [authority.certificate()] } });
  let url = format!("{}/orders", provider.base_url);

  client(trusting.clone())
    .run(get(&url), None)
    .expect_err("the provider refuses a client with no certificate");

  let identity = authority.client("verifier");
  let mut presenting = trusting;
  presenting["tls"]["client-certificate"] =
    json!({ "certificate": identity.certificate, "key": identity.key });
  let response = client(presenting)
    .run(get(&url), None)
    .expect("the provider accepts the client's certificate");
  assert_eq!(response.status(), 200);
  assert!(
    provider
      .handshakes()
      .iter()
      .any(|handshake| handshake.client_certificate)
  );
}

#[test]
fn a_proxy_carries_a_tls_request_through_a_tunnel() {
  let authority = Authority::new("Private CA");
  let provider = provider(&authority, None);
  let proxy = pact_janus_test_servers::Proxy::start();
  let authority_of = provider.base_url.trim_start_matches("https://").to_string();
  let response = client(json!({
    "tls": { "ca-certificates": [authority.certificate()] },
    "proxy": { "kind": "url", "url": proxy.url }
  }))
  .run(get(&format!("{}/orders", provider.base_url)), None)
  .expect("the proxy tunnels the request");
  assert_eq!(response.status(), 200);
  assert_eq!(proxy.carried(), vec![format!("CONNECT {authority_of} HTTP/1.1")]);
}

#[test]
fn a_host_the_proxy_is_not_for_is_reached_directly() {
  let authority = Authority::new("Private CA");
  let provider = provider(&authority, None);
  let proxy = pact_janus_test_servers::Proxy::start();
  client(json!({
    "tls": { "ca-certificates": [authority.certificate()] },
    "proxy": { "kind": "url", "url": proxy.url, "no-proxy": ["127.0.0.1"] }
  }))
  .run(get(&format!("{}/orders", provider.base_url)), None)
  .expect("the provider answers directly");
  assert_eq!(proxy.carried(), Vec::<String>::new());
}

#[test]
fn a_plain_request_goes_to_the_proxy_in_absolute_form() {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  thread::spawn(move || {
    use std::io::Write;
    let (mut stream, _) = listener.accept().unwrap();
    let mut buffer = [0u8; 1024];
    let _ = stream.read(&mut buffer);
    let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n");
  });
  let proxy = pact_janus_test_servers::Proxy::start();
  let response = client(json!({ "proxy": { "kind": "url", "url": proxy.url } }))
    .run(get(&format!("http://{address}/orders?id=1")), None)
    .expect("the proxy forwards the request");
  assert_eq!(response.status(), 204);
  assert_eq!(
    proxy.carried(),
    vec![format!("GET http://{address}/orders?id=1 HTTP/1.1")]
  );
}

#[test]
fn http_1_1_is_all_that_is_offered_by_default() {
  let authority = Authority::new("Private CA");
  let provider = provider(&authority, None);
  let trusting = json!({ "tls": { "ca-certificates": [authority.certificate()] } });
  let response = client(trusting)
    .run(get(&format!("{}/", provider.base_url)), None)
    .expect("the provider answers");
  assert_eq!(response.version(), http::Version::HTTP_11);
  assert_eq!(provider.handshakes()[0].alpn_offered, vec!["http/1.1"]);
}

#[test]
fn negotiate_offers_both_versions() {
  let authority = Authority::new("Private CA");
  let provider = provider(&authority, None);
  let response = client(json!({
    "tls": { "ca-certificates": [authority.certificate()] },
    "http-version": "negotiate"
  }))
  .run(get(&format!("{}/", provider.base_url)), None)
  .expect("a provider that speaks only HTTP/1.1 is answered in it");
  assert_eq!(response.version(), http::Version::HTTP_11);
  assert_eq!(provider.handshakes()[0].alpn_offered, vec!["h2", "http/1.1"]);
}

#[test]
fn http_2_is_spoken_with_prior_knowledge_in_the_clear() {
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  listener.set_nonblocking(true).unwrap();
  thread::spawn(move || {
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_io()
      .build()
      .unwrap();
    runtime.block_on(async move {
      let listener = tokio::net::TcpListener::from_std(listener).unwrap();
      let (socket, _) = listener.accept().await.unwrap();
      let mut connection = h2::server::handshake(socket).await.unwrap();
      while let Some(Ok((request, mut respond))) = connection.accept().await {
        let answer = http::Response::builder().status(200).body(()).unwrap();
        let mut stream = respond.send_response(answer, false).unwrap();
        let body = format!("{} over {:?}", request.uri().path(), request.version());
        stream.send_data(bytes::Bytes::from(body), true).unwrap();
      }
    });
  });
  let response = client(json!({ "http-version": "2" }))
    .run(get(&format!("http://{address}/orders")), None)
    .expect("the provider answers in HTTP/2");
  assert_eq!(response.version(), http::Version::HTTP_2);
  assert_eq!(response.body(), b"/orders over HTTP/2.0");
}

#[test]
fn a_request_that_gets_no_answer_times_out() {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  // Accepts and reads, and never answers.
  thread::spawn(move || {
    for mut stream in listener.incoming().flatten() {
      thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        while stream.read(&mut buffer).is_ok_and(|read| read > 0) {}
      });
    }
  });
  let client = client(json!({ "timeouts": { "request-ms": 200 } }));
  let started = Instant::now();
  let err = client
    .run(get(&format!("http://{address}/slow")), None)
    .expect_err("nothing answers");
  assert_eq!(err, SendError::Timeout(Duration::from_millis(200)));
  assert!(started.elapsed() < Duration::from_secs(2));

  // A shorter deadline given with the request wins.
  let err = client
    .run(
      get(&format!("http://{address}/slow")),
      Some(Duration::from_millis(50)),
    )
    .expect_err("nothing answers");
  assert_eq!(err, SendError::Timeout(Duration::from_millis(50)));
}

#[test]
fn certificates_that_do_not_parse_are_named_by_pointer() {
  let err = HttpClient::from_value(
    Some(&json!({ "tls": { "ca-certificates": ["not a certificate"] } })),
    defaults(),
  )
  .err()
  .expect("there is no certificate in it");
  assert_eq!(err.pointer.as_deref(), Some("/tls/ca-certificates/0"));

  let authority = Authority::new("Private CA");
  let identity = authority.client("verifier");
  let err = HttpClient::from_value(
    Some(&json!({ "tls": { "client-certificate": {
      "certificate": identity.certificate, "key": "-----BEGIN PRIVATE KEY-----\nnot a key\n-----END PRIVATE KEY-----"
    } } })),
    defaults(),
  )
  .err()
  .expect("the key does not parse");
  assert_eq!(err.pointer.as_deref(), Some("/tls/client-certificate/key"));
  assert!(!err.message.contains("not a key"), "{err}");
}

#[test]
fn the_document_reads_as_the_options_it_names() {
  let options = ClientOptions::from_value(Some(&json!({
    "proxy": { "kind": "none" }, "http-version": "2"
  })))
  .unwrap();
  assert_eq!(options.proxy, Proxy::None);
  assert_eq!(options.http_version, HttpVersion::Http2);
}
