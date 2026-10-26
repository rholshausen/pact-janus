//! The drive role (plan task 5.1, rewritten by staged plan S1.5): a request at the provider, its
//! answer back as parts, through the engine's one HTTP client ([`pact_janus_http_client`], ADR 0035).
//!
//! A drive instance's options (component-interfaces spec §5.6) are `base-url`, whose path prefixes
//! every request's (today's base path); `headers`, added to every request and replacing any header
//! of the same name the interaction carries (today's `--header`); and `client`, the HTTP client
//! options, which are TLS, proxies, timeouts and the HTTP version.

use crate::{BASE64, Names, multi_map_slot, query_string, slot_bytes};
use base64::Engine as _;
use pact_janus_http_client::{Defaults, HttpClient};
use pact_janus_kernel::chain;
use pact_janus_kernel::component::{ComponentError, Part, Parts, SlotValue};
use serde_json::{Map, Value};
use std::time::Duration;

/// How long a request to a provider may take when the options do not say: today's verifier's
/// default (`--request-timeout`).
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_millis(5_000);

/// The drive role's whole state: where to send, what to add, and the client, whose connections are
/// kept between requests, as today's verifier keeps them.
pub struct DriveInstance {
  base_url: String,
  headers: http::HeaderMap,
  client: HttpClient,
}

fn invalid(pointer: &str, message: impl std::fmt::Display) -> ComponentError {
  ComponentError::invalid_config("http", format!("the drive options' {pointer} {message}"))
}

impl DriveInstance {
  /// Reads a drive instance's options. Everything wrong with them is found here, so a run with a
  /// misconfigured provider fails before it starts rather than at its first request.
  pub fn from_options(options: &Map<String, Value>) -> Result<DriveInstance, ComponentError> {
    let base_url = options
      .get("base-url")
      .ok_or_else(|| invalid("/base-url", "is required"))?
      .as_str()
      .ok_or_else(|| invalid("/base-url", "must be a string"))?
      .trim_end_matches('/')
      .to_string();
    let parsed = pact_janus_http_client::uri(&base_url).map_err(|err| invalid("/base-url", err))?;
    if !matches!(parsed.scheme_str(), Some("http" | "https")) {
      return Err(invalid("/base-url", "must be an http or https URL"));
    }

    let mut headers = http::HeaderMap::new();
    match options.get("headers") {
      None => {}
      Some(Value::Object(custom)) => {
        for (name, value) in custom {
          let pointer = format!("/headers/{name}");
          let name = http::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| invalid(&pointer, "is not a header name"))?;
          // The value is a credential as often as not, so the message never repeats it.
          let value = value
            .as_str()
            .and_then(|value| http::HeaderValue::from_str(value).ok())
            .ok_or_else(|| invalid(&pointer, "must be a string a header can carry"))?;
          headers.insert(name, value);
        }
      }
      Some(_) => return Err(invalid("/headers", "must be an object of header names to values")),
    }

    let client = HttpClient::from_value(
      options.get("client"),
      Defaults {
        request_timeout: DEFAULT_REQUEST_TIMEOUT,
        // A verification observes the provider's first answer (ADR 0035 decision 3).
        retries: 0,
        user_agent: format!("pact-janus/{}", pact_janus_kernel::ENGINE_VERSION),
      },
    )
    .map_err(|err| match err.pointer {
      Some(pointer) => invalid(&format!("/client{pointer}"), err.message),
      None => ComponentError::transport_failed(err.message),
    })?;

    Ok(DriveInstance {
      base_url,
      headers,
      client,
    })
  }

  pub fn base_url(&self) -> &str {
    &self.base_url
  }

  /// One request at the provider, and its answer as a `response` part. Any status is an answer:
  /// a 404 or a 500 is what a verification run exists to observe, never a transport failure.
  pub fn send(&self, part: &Part, timeout: Option<Duration>) -> Result<Parts, ComponentError> {
    let request = self.request(part)?;
    // The HTTP library hides a URL's path below `trace`, since it cannot know which paths are
    // secret. These come from the contract, so they are logged in full, without any user
    // information the base URL carries.
    let method = request.method().to_string();
    let url = pact_janus_kernel::broker::without_userinfo(&request.uri().to_string());
    tracing::debug!(%method, %url, "sending the request to the provider");
    let response = self.client.run(request, timeout).map_err(|err| {
      tracing::debug!(%method, %url, error = %chain::sentence(&err), "the provider did not answer");
      ComponentError::transport_failed(format!("{method} {url}")).caused_by(chain::layers_of(&err, Some))
    })?;
    tracing::debug!(%method, %url, status = response.status().as_u16(), "the provider answered");
    Ok(reply_parts(response))
  }

  /// A `request` part as an HTTP request: the method, `base-url` and the path, the query slot
  /// re-encoded, every header value, then the instance's own headers over them. The slot
  /// vocabulary is the one the serve side records, so a mock's recording replays at a provider.
  fn request(&self, part: &Part) -> Result<http::Request<Vec<u8>>, ComponentError> {
    let method = part
      .get("method")
      .and_then(|slot| slot.content.as_str())
      .unwrap_or("GET")
      .to_ascii_uppercase();
    let method = http::Method::from_bytes(method.as_bytes())
      .map_err(|_| ComponentError::transport_failed(format!("'{method}' is not an HTTP method")))?;
    let path = part
      .get("path")
      .and_then(|slot| slot.content.as_str())
      .unwrap_or("/");
    let query = part.get("query").map(query_string).unwrap_or_default();
    let mut url = format!("{}{path}", self.base_url);
    if !query.is_empty() {
      url.push('?');
      url.push_str(&query);
    }
    let uri = pact_janus_http_client::uri(&url)
      .map_err(|err| ComponentError::transport_failed(format!("the request's URL {err}")))?;

    let mut headers = http::HeaderMap::new();
    if let Some(slot) = part.get("headers")
      && let Value::Object(slot) = &slot.content
    {
      for (name, values) in slot {
        let header = http::HeaderName::from_bytes(name.as_bytes())
          .map_err(|_| ComponentError::transport_failed(format!("'{name}' is not a header name")))?;
        for value in values.as_array().into_iter().flatten().filter_map(Value::as_str) {
          let value = http::HeaderValue::from_bytes(value.as_bytes()).map_err(|_| {
            ComponentError::transport_failed(format!("the value of header '{name}' cannot be sent"))
          })?;
          headers.append(header.clone(), value);
        }
      }
    }
    // The instance's headers replace the interaction's of the same name, whatever its case.
    for (name, value) in &self.headers {
      headers.insert(name.clone(), value.clone());
    }
    // The body's own content type travels with the body slot (spec §6's bytes/document boundary),
    // not in the headers slot, so it is applied here rather than lost.
    if let Some(content_type) = part.get("body").and_then(|slot| slot.content_type.as_deref())
      && !headers.contains_key(http::header::CONTENT_TYPE)
      && let Ok(value) = http::HeaderValue::from_str(content_type)
    {
      headers.insert(http::header::CONTENT_TYPE, value);
    }

    let body = match part.get("body") {
      Some(slot) => slot_bytes(slot)?,
      None => Vec::new(),
    };
    let mut request = http::Request::new(body);
    *request.method_mut() = method;
    *request.uri_mut() = uri;
    *request.headers_mut() = headers;
    Ok(request)
  }
}

/// The provider's answer as a `response` part: the same slot vocabulary the mock side records, the
/// body base64-tagged because a transport carries bytes and never decides what they mean
/// (spec §4, §5.4).
fn reply_parts(response: http::Response<Vec<u8>>) -> Parts {
  let headers: Vec<(String, String)> = response
    .headers()
    .iter()
    .map(|(name, value)| {
      (
        name.as_str().to_string(),
        String::from_utf8_lossy(value.as_bytes()).to_string(),
      )
    })
    .collect();
  let content_type = headers
    .iter()
    .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    .map(|(_, value)| value.clone());

  let mut response_part: Part = Part::new();
  response_part.insert(
    "status".to_string(),
    SlotValue {
      content: Value::from(response.status().as_u16()),
      encoded: None,
      content_type: None,
    },
  );
  response_part.insert(
    "headers".to_string(),
    multi_map_slot(
      headers.iter().map(|(n, v)| (n.as_str(), v.as_str())),
      Names::FoldCase,
    ),
  );
  response_part.insert(
    "body".to_string(),
    SlotValue {
      content: Value::String(BASE64.encode(response.body())),
      encoded: Some("base64".to_string()),
      content_type,
    },
  );

  let mut parts: Parts = Parts::new();
  parts.insert("response".to_string(), response_part);
  parts
}
