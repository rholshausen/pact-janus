//! Servers for tests that need a network (staged plan S1.5): a certificate [`Authority`] made per
//! test, a [`TlsProvider`] that can demand a client certificate and records what each client
//! offered, and a forward [`Proxy`] that records what it carried. All of them are blocking and
//! threaded, deliberately unlike the client under test, and none of them shares code with it, so
//! a test observes bytes on a wire rather than the client's own view of them.
//!
//! Test-only: a server panics on anything a test did not set up, which is a failing test.

use rcgen::{
  BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
  KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// A certificate and its private key, as PEM text (what a host passes in the HTTP client options)
/// and as DER (what a server is configured with).
#[derive(Clone)]
pub struct Identity {
  pub certificate: String,
  pub key: String,
  certificate_der: Vec<u8>,
  key_der: Vec<u8>,
}

/// A certificate authority that exists for one test: nothing trusts it unless told to.
pub struct Authority {
  issuer: Issuer<'static, KeyPair>,
  certificate: String,
  certificate_der: Vec<u8>,
}

impl Authority {
  pub fn new(name: &str) -> Authority {
    let mut params = CertificateParams::new(Vec::<String>::new()).expect("no names is valid");
    params.distinguished_name.push(DnType::CommonName, name);
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
      KeyUsagePurpose::KeyCertSign,
      KeyUsagePurpose::CrlSign,
      KeyUsagePurpose::DigitalSignature,
    ];
    let key = KeyPair::generate().expect("a key pair generates");
    let certificate = params.self_signed(&key).expect("a CA signs itself");
    Authority {
      certificate: certificate.pem(),
      certificate_der: certificate.der().to_vec(),
      issuer: Issuer::new(params, key),
    }
  }

  /// The authority's own certificate, as PEM: what a client is told to trust.
  pub fn certificate(&self) -> &str {
    &self.certificate
  }

  /// A server certificate for these names (DNS names or IP addresses).
  pub fn server(&self, names: &[&str]) -> Identity {
    let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    let mut params = CertificateParams::new(names).expect("the names are valid");
    params.distinguished_name.push(DnType::CommonName, "provider");
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    self.issue(params)
  }

  /// A client certificate, for a server that demands one.
  pub fn client(&self, name: &str) -> Identity {
    let mut params = CertificateParams::new(Vec::<String>::new()).expect("no names is valid");
    params.distinguished_name.push(DnType::CommonName, name);
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    self.issue(params)
  }

  fn issue(&self, mut params: CertificateParams) -> Identity {
    // rcgen's default validity runs to the year 4096. macOS refuses a TLS server certificate valid
    // for more than 825 days, private CA or not, so a test certificate is valid around today.
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now - time::Duration::days(1);
    params.not_after = now + time::Duration::days(30);
    let key = KeyPair::generate().expect("a key pair generates");
    let certificate = params.signed_by(&key, &self.issuer).expect("the CA signs it");
    Identity {
      certificate: certificate.pem(),
      key: key.serialize_pem(),
      certificate_der: certificate.der().to_vec(),
      key_der: key.serialize_der(),
    }
  }
}

/// One HTTP/1.1 request, as a server read it.
#[derive(Debug, Clone)]
pub struct Request {
  pub method: String,
  /// The request target, as sent: origin-form (`/orders?id=1`) to a server.
  pub target: String,
  pub headers: Vec<(String, String)>,
  pub body: Vec<u8>,
}

impl Request {
  /// The first value of a header, by name in any case.
  pub fn header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .iter()
      .find(|(header, _)| header.eq_ignore_ascii_case(name))
      .map(|(_, value)| value.as_str())
  }
}

/// What a server answers.
#[derive(Debug, Clone)]
pub struct Response {
  pub status: u16,
  pub headers: Vec<(String, String)>,
  pub body: Vec<u8>,
}

impl Response {
  pub fn json(status: u16, body: &str) -> Response {
    Response {
      status,
      headers: vec![("Content-Type".to_string(), "application/json".to_string())],
      body: body.as_bytes().to_vec(),
    }
  }
}

/// What one client did during the TLS handshake.
#[derive(Debug, Clone, Default)]
pub struct Handshake {
  /// The protocols the client offered by ALPN, in its order.
  pub alpn_offered: Vec<String>,
  /// Whether the client presented a certificate the server accepted.
  pub client_certificate: bool,
  /// Whether the handshake completed.
  pub completed: bool,
}

type Answer = dyn Fn(&Request) -> Response + Send + Sync;

/// An HTTPS provider: HTTP/1.1 over TLS, with the server certificate it is given, demanding a
/// client certificate the given authority issued when there is one.
pub struct TlsProvider {
  pub base_url: String,
  handshakes: Arc<Mutex<Vec<Handshake>>>,
  requests: Arc<Mutex<Vec<Request>>>,
}

impl TlsProvider {
  pub fn start(
    identity: &Identity,
    clients: Option<&Authority>,
    answer: impl Fn(&Request) -> Response + Send + Sync + 'static,
  ) -> TlsProvider {
    let crypto = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ServerConfig::builder_with_provider(Arc::clone(&crypto))
      .with_safe_default_protocol_versions()
      .expect("the default versions are supported");
    let builder = match clients {
      Some(authority) => {
        let mut roots = rustls::RootCertStore::empty();
        roots
          .add(CertificateDer::from(authority.certificate_der.clone()))
          .expect("the CA certificate parses");
        let verifier =
          rustls::server::WebPkiClientVerifier::builder_with_provider(Arc::new(roots), Arc::clone(&crypto))
            .build()
            .expect("the client verifier builds");
        builder.with_client_cert_verifier(verifier)
      }
      None => builder.with_no_client_auth(),
    };
    let mut config = builder
      .with_single_cert(
        vec![CertificateDer::from(identity.certificate_der.clone())],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.key_der.clone())),
      )
      .expect("the server certificate and key match");
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let config = Arc::new(config);

    let listener = TcpListener::bind("127.0.0.1:0").expect("an OS-assigned port always binds");
    let base_url = format!("https://{}", listener.local_addr().expect("bound"));
    let handshakes = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let answer: Arc<Answer> = Arc::new(answer);
    {
      let handshakes = Arc::clone(&handshakes);
      let requests = Arc::clone(&requests);
      thread::spawn(move || {
        for stream in listener.incoming().flatten() {
          let config = Arc::clone(&config);
          let handshakes = Arc::clone(&handshakes);
          let requests = Arc::clone(&requests);
          let answer = Arc::clone(&answer);
          thread::spawn(move || serve_tls(stream, config, &handshakes, &requests, answer.as_ref()));
        }
      });
    }
    TlsProvider {
      base_url,
      handshakes,
      requests,
    }
  }

  pub fn handshakes(&self) -> Vec<Handshake> {
    self.handshakes.lock().expect("not poisoned").clone()
  }

  pub fn requests(&self) -> Vec<Request> {
    self.requests.lock().expect("not poisoned").clone()
  }
}

fn serve_tls(
  mut stream: TcpStream,
  config: Arc<rustls::ServerConfig>,
  handshakes: &Mutex<Vec<Handshake>>,
  requests: &Mutex<Vec<Request>>,
  answer: &Answer,
) {
  let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
  let mut acceptor = rustls::server::Acceptor::default();
  let accepted = loop {
    if acceptor.read_tls(&mut stream).is_err() {
      return;
    }
    match acceptor.accept() {
      Ok(Some(accepted)) => break accepted,
      Ok(None) => continue,
      Err(_) => return,
    }
  };
  let alpn_offered = accepted
    .client_hello()
    .alpn()
    .map(|protocols| {
      protocols
        .map(|protocol| String::from_utf8_lossy(protocol).to_string())
        .collect()
    })
    .unwrap_or_default();
  let index = {
    let mut handshakes = handshakes.lock().expect("not poisoned");
    handshakes.push(Handshake {
      alpn_offered,
      ..Handshake::default()
    });
    handshakes.len() - 1
  };
  let Ok(connection) = accepted.into_connection(config) else {
    return;
  };
  let mut tls = rustls::StreamOwned::new(connection, stream);
  while tls.conn.is_handshaking() {
    if tls.conn.complete_io(&mut tls.sock).is_err() {
      return;
    }
  }
  {
    let mut handshakes = handshakes.lock().expect("not poisoned");
    handshakes[index].completed = true;
    handshakes[index].client_certificate = tls.conn.peer_certificates().is_some();
  }
  serve_http(&mut tls, requests, answer);
}

/// HTTP/1.1 on one connection, request after request, until the client closes it.
fn serve_http<S: Read + Write>(stream: &mut S, requests: &Mutex<Vec<Request>>, answer: &Answer) {
  let mut reader = BufReader::new(stream);
  while let Some((request_line, headers)) = read_head(&mut reader) {
    let mut words = request_line.split_whitespace();
    let method = words.next().unwrap_or_default().to_string();
    let target = words.next().unwrap_or_default().to_string();
    let length = headers
      .iter()
      .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
      .and_then(|(_, value)| value.parse().ok())
      .unwrap_or(0usize);
    let mut body = vec![0u8; length];
    if reader.read_exact(&mut body).is_err() {
      return;
    }
    let request = Request {
      method,
      target,
      headers,
      body,
    };
    let response = answer(&request);
    requests.lock().expect("not poisoned").push(request);

    let mut head = format!("HTTP/1.1 {} Answer\r\n", response.status);
    for (name, value) in &response.headers {
      head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n", response.body.len()));
    let stream = reader.get_mut();
    if stream.write_all(head.as_bytes()).is_err()
      || stream.write_all(&response.body).is_err()
      || stream.flush().is_err()
    {
      return;
    }
  }
}

/// A request or response head: its first line and its headers, or `None` at the end of the stream.
fn read_head<R: BufRead>(reader: &mut R) -> Option<(String, Vec<(String, String)>)> {
  let mut first = String::new();
  if reader.read_line(&mut first).ok()? == 0 {
    return None;
  }
  let mut headers = Vec::new();
  loop {
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
      return None;
    }
    let line = line.trim_end();
    if line.is_empty() {
      break;
    }
    let (name, value) = line.split_once(':')?;
    headers.push((name.trim().to_string(), value.trim().to_string()));
  }
  Some((first.trim_end().to_string(), headers))
}

/// A forward proxy: it tunnels `CONNECT host:port` and forwards an absolute-form request
/// (`GET http://host:port/path`), recording each as its first line.
pub struct Proxy {
  pub url: String,
  carried: Arc<Mutex<Vec<String>>>,
}

impl Proxy {
  pub fn start() -> Proxy {
    let listener = TcpListener::bind("127.0.0.1:0").expect("an OS-assigned port always binds");
    let url = format!("http://{}", listener.local_addr().expect("bound"));
    let carried = Arc::new(Mutex::new(Vec::new()));
    {
      let carried = Arc::clone(&carried);
      thread::spawn(move || {
        for stream in listener.incoming().flatten() {
          let carried = Arc::clone(&carried);
          thread::spawn(move || carry(stream, &carried));
        }
      });
    }
    Proxy { url, carried }
  }

  /// The first line of everything the proxy carried, in the order it arrived.
  pub fn carried(&self) -> Vec<String> {
    self.carried.lock().expect("not poisoned").clone()
  }
}

fn carry(client: TcpStream, carried: &Mutex<Vec<String>>) {
  let _ = client.set_read_timeout(Some(Duration::from_secs(10)));
  let mut reader = BufReader::new(client);
  let Some((first, headers)) = read_head(&mut reader) else {
    return;
  };
  carried.lock().expect("not poisoned").push(first.clone());
  let mut words = first.split_whitespace();
  let method = words.next().unwrap_or_default().to_string();
  let target = words.next().unwrap_or_default().to_string();
  let version = words.next().unwrap_or("HTTP/1.1").to_string();

  if method.eq_ignore_ascii_case("CONNECT") {
    let Ok(upstream) = TcpStream::connect(&target) else {
      let _ = reader
        .get_mut()
        .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n");
      return;
    };
    if reader
      .get_mut()
      .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
      .is_err()
    {
      return;
    }
    pipe(reader, upstream);
    return;
  }

  // Absolute form: http://host:port/path?query, sent on in origin form.
  let Some(rest) = target.strip_prefix("http://") else {
    let _ = reader
      .get_mut()
      .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
    return;
  };
  let (authority, path) = match rest.find('/') {
    Some(slash) => (&rest[..slash], &rest[slash..]),
    None => (rest, "/"),
  };
  let Ok(mut upstream) = TcpStream::connect(authority) else {
    let _ = reader
      .get_mut()
      .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n");
    return;
  };
  let mut head = format!("{method} {path} {version}\r\n");
  for (name, value) in headers {
    if !name.eq_ignore_ascii_case("proxy-connection") {
      head.push_str(&format!("{name}: {value}\r\n"));
    }
  }
  head.push_str("\r\n");
  if upstream.write_all(head.as_bytes()).is_err() {
    return;
  }
  pipe(reader, upstream);
}

/// Copies bytes both ways until either side closes, starting with whatever the client already sent
/// past the head.
fn pipe(client: BufReader<TcpStream>, mut upstream: TcpStream) {
  let buffered = client.buffer().to_vec();
  if !buffered.is_empty() && upstream.write_all(&buffered).is_err() {
    return;
  }
  let client = client.into_inner();
  let (Ok(mut client_reader), Ok(mut upstream_reader)) = (client.try_clone(), upstream.try_clone()) else {
    return;
  };
  let mut client_writer = client;
  let upward = thread::spawn(move || {
    let _ = std::io::copy(&mut client_reader, &mut upstream);
    let _ = upstream.shutdown(std::net::Shutdown::Write);
  });
  let _ = std::io::copy(&mut upstream_reader, &mut client_writer);
  let _ = client_writer.shutdown(std::net::Shutdown::Write);
  let _ = upward.join();
}
