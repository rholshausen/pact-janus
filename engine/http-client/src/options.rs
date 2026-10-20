//! The HTTP client options document (component-interfaces spec §5.6, ADR 0035 decision 3): the
//! `client` member of the HTTP transport's drive options and of the broker descriptor.
//!
//! An open document, as every document the protocol carries is: a member this client does not know
//! is ignored, and one it knows with the wrong type is an error naming it by pointer, so a typo in a
//! known member is never silently a default.

use serde_json::{Map, Value};
use std::fmt;
use std::time::Duration;

/// What is wrong with an options document, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsError {
  /// A JSON pointer into the options document.
  pub pointer: String,
  pub message: String,
}

impl fmt::Display for OptionsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}: {}", self.pointer, self.message)
  }
}

impl std::error::Error for OptionsError {}

fn error(pointer: &str, message: impl Into<String>) -> OptionsError {
  OptionsError {
    pointer: pointer.to_string(),
    message: message.into(),
  }
}

/// A client certificate and its private key, as PEM text. The key is a credential: it is never
/// formatted, logged or repeated in an error.
#[derive(Clone, PartialEq, Eq)]
pub struct ClientCertificate {
  pub certificate: String,
  pub key: String,
}

impl fmt::Debug for ClientCertificate {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ClientCertificate")
      .field("certificate", &format_args!("<{} bytes>", self.certificate.len()))
      .field("key", &format_args!("***"))
      .finish()
  }
}

/// Where a request goes on its way to the server.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Proxy {
  /// `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` and `NO_PROXY` where the engine runs, as today's
  /// verifier and curl read them.
  #[default]
  Environment,
  /// Straight to the server, whatever the environment says.
  None,
  /// Through this proxy, except to the hosts `no_proxy` names (`NO_PROXY`'s syntax).
  Url { url: String, no_proxy: Vec<String> },
}

/// Which HTTP versions the client speaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HttpVersion {
  /// HTTP/1.1 only: today's verifier is built without HTTP/2.
  #[default]
  Http1,
  /// HTTP/2 only: offered by ALPN over TLS, and with prior knowledge in the clear.
  Http2,
  /// ALPN offers both over TLS; HTTP/1.1 in the clear.
  Negotiate,
}

/// The options document, read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientOptions {
  /// PEM text, each possibly a bundle, trusted beside the platform's roots.
  pub ca_certificates: Vec<String>,
  pub client_certificate: Option<ClientCertificate>,
  /// Accept any server certificate (today's `--disable-ssl-verification`).
  pub insecure: bool,
  pub proxy: Proxy,
  pub request_timeout: Option<Duration>,
  pub connect_timeout: Option<Duration>,
  pub http_version: HttpVersion,
  /// How many times a client that retries tries again. A provider request is never retried.
  pub retries: Option<u32>,
}

impl ClientOptions {
  /// Reads the document. `None` (no `client` member) is every default.
  pub fn from_value(value: Option<&Value>) -> Result<ClientOptions, OptionsError> {
    let Some(value) = value else {
      return Ok(ClientOptions::default());
    };
    let document = object(value, "")?;
    let mut options = ClientOptions::default();

    if let Some(tls) = document.get("tls") {
      let tls = object(tls, "/tls")?;
      if let Some(cas) = tls.get("ca-certificates") {
        let Value::Array(cas) = cas else {
          return Err(error("/tls/ca-certificates", "must be an array of PEM text"));
        };
        for (index, ca) in cas.iter().enumerate() {
          options
            .ca_certificates
            .push(text(ca, &format!("/tls/ca-certificates/{index}"))?);
        }
      }
      if let Some(certificate) = tls.get("client-certificate") {
        let pair = object(certificate, "/tls/client-certificate")?;
        let member = |name: &str| {
          let pointer = format!("/tls/client-certificate/{name}");
          pair
            .get(name)
            .ok_or_else(|| error(&pointer, "is required"))
            .and_then(|value| text(value, &pointer))
        };
        options.client_certificate = Some(ClientCertificate {
          certificate: member("certificate")?,
          key: member("key")?,
        });
      }
      if let Some(insecure) = tls.get("insecure") {
        options.insecure = insecure
          .as_bool()
          .ok_or_else(|| error("/tls/insecure", "must be true or false"))?;
      }
    }

    if let Some(proxy) = document.get("proxy") {
      let proxy = object(proxy, "/proxy")?;
      options.proxy = match proxy.get("kind").and_then(Value::as_str) {
        None | Some("environment") => Proxy::Environment,
        Some("none") => Proxy::None,
        Some("url") => {
          let url = proxy
            .get("url")
            .ok_or_else(|| error("/proxy/url", "is required when kind is 'url'"))
            .and_then(|value| text(value, "/proxy/url"))?;
          let no_proxy = match proxy.get("no-proxy") {
            None => Vec::new(),
            Some(Value::Array(hosts)) => hosts
              .iter()
              .enumerate()
              .map(|(index, host)| text(host, &format!("/proxy/no-proxy/{index}")))
              .collect::<Result<_, _>>()?,
            Some(_) => return Err(error("/proxy/no-proxy", "must be an array of hosts")),
          };
          Proxy::Url { url, no_proxy }
        }
        Some(other) => {
          return Err(error(
            "/proxy/kind",
            format!("'{other}' is not a proxy kind this client knows (environment, none, url)"),
          ));
        }
      };
    }

    if let Some(timeouts) = document.get("timeouts") {
      let timeouts = object(timeouts, "/timeouts")?;
      options.request_timeout = milliseconds(timeouts.get("request-ms"), "/timeouts/request-ms")?;
      options.connect_timeout = milliseconds(timeouts.get("connect-ms"), "/timeouts/connect-ms")?;
    }

    if let Some(version) = document.get("http-version") {
      options.http_version = match version.as_str() {
        Some("1.1") => HttpVersion::Http1,
        Some("2") => HttpVersion::Http2,
        Some("negotiate") => HttpVersion::Negotiate,
        _ => {
          return Err(error(
            "/http-version",
            format!("{version} is not an HTTP version this client speaks (\"1.1\", \"2\", \"negotiate\")"),
          ));
        }
      };
    }

    if let Some(retries) = document.get("retries") {
      let count = retries
        .as_u64()
        .and_then(|count| u32::try_from(count).ok())
        .ok_or_else(|| error("/retries", "must be a whole number, zero or more"))?;
      options.retries = Some(count);
    }

    Ok(options)
  }
}

fn object<'a>(value: &'a Value, pointer: &str) -> Result<&'a Map<String, Value>, OptionsError> {
  value.as_object().ok_or_else(|| {
    error(
      if pointer.is_empty() { "/" } else { pointer },
      "must be an object",
    )
  })
}

fn text(value: &Value, pointer: &str) -> Result<String, OptionsError> {
  value
    .as_str()
    .map(str::to_string)
    .ok_or_else(|| error(pointer, "must be a string"))
}

/// A timeout: a whole number of milliseconds above zero. Zero is refused rather than read as "no
/// timeout": a run must end when a provider hangs.
fn milliseconds(value: Option<&Value>, pointer: &str) -> Result<Option<Duration>, OptionsError> {
  match value {
    None => Ok(None),
    Some(value) => match value.as_u64() {
      Some(ms) if ms > 0 => Ok(Some(Duration::from_millis(ms))),
      _ => Err(error(
        pointer,
        "must be a whole number of milliseconds above zero",
      )),
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use rstest::rstest;
  use serde_json::json;

  #[test]
  fn no_document_is_every_default() {
    assert_eq!(ClientOptions::from_value(None).unwrap(), ClientOptions::default());
    assert_eq!(
      ClientOptions::from_value(Some(&json!({}))).unwrap(),
      ClientOptions::default()
    );
  }

  #[test]
  fn reads_every_member() {
    let options = ClientOptions::from_value(Some(&json!({
      "tls": {
        "ca-certificates": ["CA"],
        "client-certificate": { "certificate": "CERT", "key": "KEY" },
        "insecure": true
      },
      "proxy": { "kind": "url", "url": "http://proxy:3128", "no-proxy": ["localhost"] },
      "timeouts": { "request-ms": 1500, "connect-ms": 200 },
      "http-version": "negotiate",
      "retries": 4,
      "a-member-from-a-later-version": true
    })))
    .unwrap();
    assert_eq!(
      options,
      ClientOptions {
        ca_certificates: vec!["CA".to_string()],
        client_certificate: Some(ClientCertificate {
          certificate: "CERT".to_string(),
          key: "KEY".to_string()
        }),
        insecure: true,
        proxy: Proxy::Url {
          url: "http://proxy:3128".to_string(),
          no_proxy: vec!["localhost".to_string()]
        },
        request_timeout: Some(Duration::from_millis(1500)),
        connect_timeout: Some(Duration::from_millis(200)),
        http_version: HttpVersion::Negotiate,
        retries: Some(4),
      }
    );
  }

  #[rstest]
  #[case(json!({ "proxy": { "kind": "none" } }), Proxy::None)]
  #[case(json!({ "proxy": { "kind": "environment" } }), Proxy::Environment)]
  #[case(json!({ "proxy": {} }), Proxy::Environment)]
  fn reads_a_proxy_kind(#[case] document: Value, #[case] expected: Proxy) {
    assert_eq!(
      ClientOptions::from_value(Some(&document)).unwrap().proxy,
      expected
    );
  }

  #[rstest]
  #[case(json!([]), "/")]
  #[case(json!({ "tls": { "ca-certificates": "CA" } }), "/tls/ca-certificates")]
  #[case(json!({ "tls": { "client-certificate": { "certificate": "CERT" } } }), "/tls/client-certificate/key")]
  #[case(json!({ "tls": { "insecure": "yes" } }), "/tls/insecure")]
  #[case(json!({ "proxy": { "kind": "url" } }), "/proxy/url")]
  #[case(json!({ "proxy": { "kind": "socks" } }), "/proxy/kind")]
  #[case(json!({ "timeouts": { "request-ms": 0 } }), "/timeouts/request-ms")]
  #[case(json!({ "timeouts": { "connect-ms": "1s" } }), "/timeouts/connect-ms")]
  #[case(json!({ "http-version": 2 }), "/http-version")]
  #[case(json!({ "retries": -1 }), "/retries")]
  fn names_the_member_that_is_wrong(#[case] document: Value, #[case] pointer: &str) {
    assert_eq!(
      ClientOptions::from_value(Some(&document)).unwrap_err().pointer,
      pointer
    );
  }

  #[test]
  fn never_shows_a_key() {
    let certificate = ClientCertificate {
      certificate: "CERT".to_string(),
      key: "-----BEGIN PRIVATE KEY-----secret".to_string(),
    };
    assert!(!format!("{certificate:?}").contains("secret"));
  }
}
