//! The native HTTP client (ADR 0035, staged plan S1.5): every request the engine sends out of the
//! process goes through an [`HttpClient`], the HTTP transport's requests to a provider and the
//! broker client's to a Pact Broker alike, configured by one document, the HTTP client options
//! ([`ClientOptions`], component-interfaces spec §5.6).
//!
//! It is today's verifier's client: reqwest on rustls with the ring provider and the platform
//! verifier, proxies from the environment, redirects followed, HTTP/1.1 unless asked otherwise, so
//! a provider moving to Janus is reached the way it was before.
//!
//! **Async and thread-free, behind an executor seam** (ADR 0035 decision 2). [`HttpClient::send`]
//! is an `async fn` from an [`http::Request`] to an [`http::Response`], and nothing above it knows
//! which runtime runs it. Each client owns an [`Executor`], a current-thread tokio runtime on native
//! targets, which starts no thread, and names are resolved inside the request's own future, since
//! reqwest's default resolver starts a thread for each lookup. The synchronous traits the kernel
//! calls block on the executor ([`HttpClient::run`]). On `wasm32-wasip3` (gate W), the executor
//! becomes WASI's and `send` becomes `wasi:http`'s, where TLS and proxies are the host's: those two
//! are the seam.

mod options;

pub use options::{ClientCertificate, ClientOptions, HttpVersion, OptionsError, Proxy};

use std::fmt;
use std::future::Future;
use std::net::ToSocketAddrs;
use std::time::Duration;

/// Runs the client's futures. On native targets, a current-thread tokio runtime: it drives every
/// connection, timer and resolution on the thread that calls [`Executor::block_on`], and starts no
/// thread of its own.
pub struct Executor {
  runtime: Option<tokio::runtime::Runtime>,
}

impl Executor {
  pub fn new() -> std::io::Result<Executor> {
    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_io()
      .enable_time()
      .build()?;
    Ok(Executor {
      runtime: Some(runtime),
    })
  }

  /// Runs `future` to completion on the calling thread. Inside another async runtime that would
  /// block that runtime's thread, which tokio refuses with a panic; a panic must never cross the
  /// engine boundary, so this answers `None` instead.
  pub fn block_on<F: Future>(&self, future: F) -> Option<F::Output> {
    if tokio::runtime::Handle::try_current().is_ok() {
      return None;
    }
    self.runtime.as_ref().map(|runtime| runtime.block_on(future))
  }
}

impl Drop for Executor {
  /// Dropping a runtime waits for its tasks, which tokio refuses inside another runtime. A client's
  /// tasks are its idle connections, so they are let go rather than waited for.
  fn drop(&mut self) {
    if let Some(runtime) = self.runtime.take() {
      runtime.shutdown_background();
    }
  }
}

/// Resolves a name inside the request's future, on the executor's own thread, where reqwest's
/// default resolver would start a thread for each lookup. A lookup blocks the executor while it
/// runs; a client sends one request at a time, so nothing else is waiting on it.
struct InlineResolver;

impl reqwest::dns::Resolve for InlineResolver {
  fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
    let host = name.as_str().to_string();
    Box::pin(async move {
      let addresses: Vec<_> = (host.as_str(), 0).to_socket_addrs()?.collect();
      Ok(Box::new(addresses.into_iter()) as reqwest::dns::Addrs)
    })
  }
}

/// What a client does when its options document does not say: each use of the client has its own
/// (a provider's request timeout is today's verifier's 5 s, a broker's the S1.3 client's 60 s).
#[derive(Debug, Clone)]
pub struct Defaults {
  pub request_timeout: Duration,
  pub retries: u32,
  /// Sent unless a request carries its own `User-Agent`.
  pub user_agent: String,
}

/// Why a client could not be built from its options. Never repeats a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientError {
  /// A JSON pointer into the options document, when one member is to blame.
  pub pointer: Option<String>,
  pub message: String,
}

impl fmt::Display for ClientError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self.pointer {
      Some(pointer) => write!(f, "{pointer}: {}", self.message),
      None => f.write_str(&self.message),
    }
  }
}

impl std::error::Error for ClientError {}

impl From<OptionsError> for ClientError {
  fn from(err: OptionsError) -> Self {
    ClientError {
      pointer: Some(err.pointer),
      message: err.message,
    }
  }
}

/// Why a request got no answer. An answer of any status is not an error: a 404 or a 500 is what a
/// verification exists to observe.
///
/// As Rust's errors conventionally are, it says only its own layer, and its causes are its
/// [`source`](std::error::Error::source) chain (ADR 0054): a caller that builds an error chain reads
/// them one by one, and one that wants a sentence joins them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
  /// Nothing answered within the request's timeout.
  Timeout(Duration),
  /// No connection: refused, unresolvable, or the TLS handshake failed. Why, beneath it.
  Connect(Option<Box<Cause>>),
  /// Anything else: a request that could not be built, a connection that broke mid-answer. Its
  /// own words are the cause's outermost layer.
  Other(Box<Cause>),
}

impl fmt::Display for SendError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SendError::Timeout(timeout) => write!(f, "no answer within {} ms", timeout.as_millis()),
      SendError::Connect(_) => f.write_str("could not connect"),
      SendError::Other(cause) => f.write_str(&cause.message),
    }
  }
}

impl std::error::Error for SendError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      SendError::Timeout(_) => None,
      SendError::Connect(cause) => cause.as_deref().map(|cause| cause as _),
      SendError::Other(cause) => cause.source.as_deref().map(|cause| cause as _),
    }
  }
}

/// One cause beneath a [`SendError`], in its own words, with its own cause beneath it. Only the
/// layers that say why are kept: the ones that only name the library in the way are not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cause {
  message: String,
  source: Option<Box<Cause>>,
}

impl Cause {
  /// `layers`, outermost first, as a chain.
  fn chain(layers: Vec<String>) -> Option<Box<Cause>> {
    layers
      .into_iter()
      .rev()
      .fold(None, |source, message| Some(Box::new(Cause { message, source })))
  }

  /// `message`, caused by `layers`.
  fn of(message: impl Into<String>, layers: Vec<String>) -> Box<Cause> {
    Box::new(Cause {
      message: message.into(),
      source: Cause::chain(layers),
    })
  }

  /// `layers` as an error whose own words are the first of them.
  fn layers(layers: Vec<String>) -> Box<Cause> {
    Cause::chain(layers).unwrap_or_else(|| Cause::of("the request failed", Vec::new()))
  }
}

impl fmt::Display for Cause {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}

impl std::error::Error for Cause {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    self.source.as_deref().map(|cause| cause as _)
  }
}

/// An error and every cause beneath it, outermost first, without the URL reqwest would put in front:
/// the caller names the request in its own terms, without any user information. The layers that
/// only say which library was in the way — reqwest's "error sending request", hyper's "client error
/// (Connect)", its connector's "tcp connect error" — are left out, so what remains is the cause:
/// `Connection refused`.
fn describe(err: reqwest::Error) -> Vec<String> {
  let err = err.without_url();
  let mut layers: Vec<String> = Vec::new();
  let mut cause: Option<&dyn std::error::Error> = Some(&err);
  while let Some(error) = cause {
    let text = without_os_error(error.to_string());
    // Some layers repeat their cause in their own message; say it once.
    if !layers.iter().any(|layer| layer.contains(&text)) {
      layers.push(text);
    }
    cause = error.source();
  }
  let wrapper = |layer: &String| {
    layer == "error sending request" || layer == "tcp connect error" || layer.starts_with("client error (")
  };
  let kept: Vec<String> = layers.iter().filter(|layer| !wrapper(layer)).cloned().collect();
  if kept.is_empty() { layers } else { kept }
}

/// [`describe`], as one sentence: what a [`ClientError`] says, since a configuration that cannot be
/// used is a position and a reason, not a chain of causes.
fn described(err: reqwest::Error) -> String {
  describe(err).join(": ")
}

/// `Connection refused`, without the `(os error 111)` the platform's own number adds.
fn without_os_error(text: String) -> String {
  match text.rfind(" (os error ") {
    Some(at) if text.ends_with(')') => text[..at].to_string(),
    _ => text,
  }
}

/// A URL as today's verifier reads one, by the WHATWG URL standard, which percent-encodes what a
/// URI cannot carry (a space in a path, say) and resolves `.` and `..` segments, as a request's
/// URI. The message never repeats the URL, which may carry user information.
pub fn uri(text: &str) -> Result<http::Uri, String> {
  let url = reqwest::Url::parse(text).map_err(|err| format!("is not a URL: {err}"))?;
  http::Uri::try_from(url.as_str()).map_err(|err| format!("is not a URI: {err}"))
}

/// A client certificate and key read as TLS will read them, so a key that cannot sign is named
/// here, by its member, rather than as a client that could not be built. The messages say what is
/// wrong with the PEM, never what is in it.
fn check_identity(identity: &ClientCertificate) -> Result<(), ClientError> {
  use rustls::pki_types::pem::PemObject as _;
  use rustls::pki_types::{CertificateDer, PrivateKeyDer};
  let wrong = |member: &str, message: &str| ClientError {
    pointer: Some(format!("/tls/client-certificate/{member}")),
    message: message.to_string(),
  };
  let certificates: Vec<_> = CertificateDer::pem_slice_iter(identity.certificate.as_bytes())
    .collect::<Result<_, _>>()
    .map_err(|_| wrong("certificate", "is not PEM certificates"))?;
  if certificates.is_empty() {
    return Err(wrong("certificate", "holds no PEM certificate"));
  }
  let key = PrivateKeyDer::from_pem_slice(identity.key.as_bytes())
    .map_err(|_| wrong("key", "is not a PEM private key"))?;
  rustls::crypto::ring::sign::any_supported_type(&key)
    .map_err(|_| wrong("key", "is not a private key TLS can sign with"))?;
  Ok(())
}

/// One configured client: its executor, its connection pool, and what it does by default.
pub struct HttpClient {
  executor: Executor,
  client: reqwest::Client,
  request_timeout: Duration,
  retries: u32,
}

impl HttpClient {
  /// Builds a client from the options document (`None` when there is no `client` member).
  pub fn from_value(
    options: Option<&serde_json::Value>,
    defaults: Defaults,
  ) -> Result<HttpClient, ClientError> {
    HttpClient::new(&ClientOptions::from_value(options)?, defaults)
  }

  pub fn new(options: &ClientOptions, defaults: Defaults) -> Result<HttpClient, ClientError> {
    // reqwest built without a provider refuses to build a client unless one is installed. Ring is
    // today's verifier's, and installing it is what today's verifier does.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
      let _ = rustls::crypto::ring::default_provider().install_default();
    }

    let request_timeout = options.request_timeout.unwrap_or(defaults.request_timeout);
    let mut builder = reqwest::Client::builder()
      .user_agent(defaults.user_agent)
      .dns_resolver(InlineResolver)
      .timeout(request_timeout)
      .tls_danger_accept_invalid_certs(options.insecure);
    if let Some(timeout) = options.connect_timeout {
      builder = builder.connect_timeout(timeout);
    }

    for (index, pem) in options.ca_certificates.iter().enumerate() {
      let pointer = format!("/tls/ca-certificates/{index}");
      let certificates =
        reqwest::Certificate::from_pem_bundle(pem.as_bytes()).map_err(|err| ClientError {
          pointer: Some(pointer.clone()),
          message: format!("is not PEM certificates: {}", described(err)),
        })?;
      if certificates.is_empty() {
        return Err(ClientError {
          pointer: Some(pointer),
          message: "holds no PEM certificate".to_string(),
        });
      }
      builder = builder.tls_certs_merge(certificates);
    }

    if let Some(certificate) = &options.client_certificate {
      check_identity(certificate)?;
      let pem = format!("{}\n{}", certificate.certificate, certificate.key);
      // The parser's own message is about structure, never the bytes, and the key is not repeated.
      let identity = reqwest::Identity::from_pem(pem.as_bytes()).map_err(|err| ClientError {
        pointer: Some("/tls/client-certificate".to_string()),
        message: format!("is not a PEM certificate and private key: {}", described(err)),
      })?;
      builder = builder.identity(identity);
    }

    builder = match &options.proxy {
      Proxy::Environment => builder,
      Proxy::None => builder.no_proxy(),
      Proxy::Url { url, no_proxy } => {
        let mut proxy = reqwest::Proxy::all(url).map_err(|err| ClientError {
          pointer: Some("/proxy/url".to_string()),
          message: format!("is not a proxy URL: {}", described(err)),
        })?;
        if !no_proxy.is_empty() {
          proxy = proxy.no_proxy(reqwest::NoProxy::from_string(&no_proxy.join(",")));
        }
        builder.proxy(proxy)
      }
    };

    builder = match options.http_version {
      HttpVersion::Http1 => builder.http1_only(),
      HttpVersion::Http2 => builder.http2_prior_knowledge(),
      HttpVersion::Negotiate => builder,
    };

    let client = builder.build().map_err(|err| ClientError {
      pointer: None,
      message: format!("the HTTP client could not be built: {}", described(err)),
    })?;
    let executor = Executor::new().map_err(|err| ClientError {
      pointer: None,
      message: format!("the HTTP client's executor could not start: {err}"),
    })?;
    Ok(HttpClient {
      executor,
      client,
      request_timeout,
      retries: options.retries.unwrap_or(defaults.retries),
    })
  }

  /// How long a request may take, unless a shorter deadline is given with it.
  pub fn request_timeout(&self) -> Duration {
    self.request_timeout
  }

  /// How many times a caller that retries should try again.
  pub fn retries(&self) -> u32 {
    self.retries
  }

  /// Sends one request and reads the whole answer. The request is bounded by the client's request
  /// timeout, or by `timeout` when that is shorter.
  pub async fn send(
    &self,
    request: http::Request<Vec<u8>>,
    timeout: Option<Duration>,
  ) -> Result<http::Response<Vec<u8>>, SendError> {
    let deadline = timeout.map_or(self.request_timeout, |timeout| timeout.min(self.request_timeout));
    let mut request = reqwest::Request::try_from(request)
      .map_err(|err| SendError::Other(Cause::of("the request could not be built", describe(err))))?;
    *request.timeout_mut() = Some(deadline);

    let classify = |err: reqwest::Error| {
      if err.is_timeout() {
        SendError::Timeout(deadline)
      } else if err.is_connect() {
        SendError::Connect(Cause::chain(describe(err)))
      } else {
        SendError::Other(Cause::layers(describe(err)))
      }
    };
    let response = self.client.execute(request).await.map_err(classify)?;
    let status = response.status();
    let version = response.version();
    let headers = response.headers().clone();
    let body = response.bytes().await.map_err(classify)?;

    let mut answer = http::Response::new(body.to_vec());
    *answer.status_mut() = status;
    *answer.version_mut() = version;
    *answer.headers_mut() = headers;
    Ok(answer)
  }

  /// [`HttpClient::send`], blocking the calling thread on the client's executor.
  pub fn run(
    &self,
    request: http::Request<Vec<u8>>,
    timeout: Option<Duration>,
  ) -> Result<http::Response<Vec<u8>>, SendError> {
    self
      .executor
      .block_on(self.send(request, timeout))
      .unwrap_or_else(|| {
        Err(SendError::Other(Cause::of(
          "the HTTP client was called from inside an async runtime, which it would block",
          Vec::new(),
        )))
      })
  }
}
