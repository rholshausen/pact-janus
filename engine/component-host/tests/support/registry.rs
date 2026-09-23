//! An OCI registry small enough to read (plan task 8.2's tests): the distribution-API calls a push
//! and a pull make, in memory, on a loopback port — and able to misbehave on request, which is what
//! a real registry cannot be asked to do. It lies about content (`tamper`), demands a token
//! (`require_token`), and keeps a log of every request, so a test can say "the second run made
//! none".
//!
//! The real registry — `registry:2`, in CI — is the other half: `tests/oci.rs` runs against it when
//! `JANUS_OCI_REGISTRY` names one, because a test registry that agrees with the client by
//! construction proves only that they agree.

#![allow(dead_code)]

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::{Header, Method, Response, Server};

/// How the registry lies, when asked to.
#[derive(Clone, Debug, PartialEq)]
pub enum Tamper {
  None,
  /// Serve a manifest with one byte changed, whatever was asked for.
  Manifests,
  /// Serve every blob with one byte changed.
  Blobs,
}

#[derive(Default)]
struct State {
  blobs: HashMap<String, Vec<u8>>,
  /// `repository:reference` → (content type, bytes); a reference is a tag or a digest.
  manifests: HashMap<String, (String, Vec<u8>)>,
  uploads: HashMap<String, Vec<u8>>,
  log: Vec<String>,
  tamper: Option<Tamper>,
  require_token: bool,
}

pub struct Registry {
  pub host: String,
  state: Arc<Mutex<State>>,
  server: Arc<Server>,
  worker: Option<thread::JoinHandle<()>>,
}

const TOKEN: &str = "a-token-the-registry-issued";

fn sha256(bytes: &[u8]) -> String {
  format!("sha256:{:x}", Sha256::digest(bytes))
}

/// One response type for every arm: tiny_http's `empty` and `from_data` are different types.
fn status(code: u16) -> Response<std::io::Cursor<Vec<u8>>> {
  Response::from_data(Vec::new()).with_status_code(code)
}

fn header(name: &str, value: &str) -> Header {
  Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("a valid header")
}

impl Registry {
  pub fn start() -> Registry {
    let server = Arc::new(Server::http("127.0.0.1:0").expect("a loopback port"));
    let host = format!("127.0.0.1:{}", server.server_addr().to_ip().unwrap().port());
    let state = Arc::new(Mutex::new(State::default()));
    let (serving, shared, own_host) = (Arc::clone(&server), Arc::clone(&state), host.clone());
    let worker = thread::spawn(move || {
      static UPLOADS: AtomicU64 = AtomicU64::new(0);
      for mut request in serving.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        let mut body = Vec::new();
        let _ = request.as_reader().read_to_end(&mut body);
        let authorization = request
          .headers()
          .iter()
          .find(|h| h.field.equiv("authorization"))
          .map(|h| h.value.to_string());
        let mut state = shared.lock().unwrap();
        state.log.push(format!("{method} {url}"));

        let (path, query) = url.split_once('?').unwrap_or((&url, ""));
        if path == "/token" {
          let _ = request.respond(Response::from_string(format!("{{\"token\":\"{TOKEN}\"}}")));
          continue;
        }
        if state.require_token && authorization.as_deref() != Some(&format!("Bearer {TOKEN}")) {
          let challenge = format!(
            "Bearer realm=\"http://{own_host}/token\",service=\"test-registry\",scope=\"repository:anything:pull,push\""
          );
          let _ = request.respond(
            Response::from_string("{\"errors\":[{\"code\":\"UNAUTHORIZED\"}]}")
              .with_status_code(401)
              .with_header(header("www-authenticate", &challenge)),
          );
          continue;
        }

        let Some(rest) = path.strip_prefix("/v2/") else {
          let _ = request.respond(status(404));
          continue;
        };
        let tamper = state.tamper.clone().unwrap_or(Tamper::None);
        let response = if let Some((repository, id)) = rest.split_once("/blobs/uploads/") {
          match method {
            Method::Post => {
              let id = UPLOADS.fetch_add(1, Ordering::Relaxed).to_string();
              state.uploads.insert(id.clone(), Vec::new());
              status(202).with_header(header(
                "location",
                &format!("/v2/{repository}/blobs/uploads/{id}?state=opaque"),
              ))
            }
            Method::Put => {
              let digest = query
                .split('&')
                .find_map(|pair| pair.strip_prefix("digest="))
                .unwrap_or_default()
                .replace("%3A", ":");
              let mut bytes = state.uploads.remove(id).unwrap_or_default();
              bytes.extend(body);
              if sha256(&bytes) != digest {
                status(400)
              } else {
                state.blobs.insert(digest, bytes);
                status(201)
              }
            }
            _ => status(405),
          }
        } else if let Some((_repository, digest)) = rest.split_once("/blobs/") {
          match state.blobs.get(digest) {
            Some(_) if method == Method::Head => status(200),
            Some(bytes) => {
              let mut bytes = bytes.clone();
              if tamper == Tamper::Blobs {
                bytes[0] ^= 1;
              }
              Response::from_data(bytes)
            }
            None => status(404),
          }
        } else if let Some((repository, reference)) = rest.split_once("/manifests/") {
          let key = format!("{repository}:{reference}");
          match method {
            Method::Put => {
              let content_type = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("content-type"))
                .map(|h| h.value.to_string())
                .unwrap_or_default();
              let digest = sha256(&body);
              state.manifests.insert(
                format!("{repository}:{digest}"),
                (content_type.clone(), body.clone()),
              );
              state.manifests.insert(key, (content_type, body));
              status(201).with_header(header("docker-content-digest", &digest))
            }
            _ => match state.manifests.get(&key) {
              Some((content_type, bytes)) => {
                let mut bytes = bytes.clone();
                if tamper == Tamper::Manifests {
                  // Still JSON, still a manifest — just not the one that digest names.
                  bytes.push(b'\n');
                }
                Response::from_data(bytes).with_header(header("content-type", content_type))
              }
              None => status(404),
            },
          }
        } else {
          status(404)
        };
        drop(state);
        let _ = request.respond(response);
      }
    });
    Registry {
      host,
      state,
      server,
      worker: Some(worker),
    }
  }

  pub fn tamper(&self, tamper: Tamper) {
    self.state.lock().unwrap().tamper = Some(tamper);
  }

  pub fn require_token(&self) {
    self.state.lock().unwrap().require_token = true;
  }

  /// Every request since the last call, as `METHOD /path?query`.
  pub fn take_log(&self) -> Vec<String> {
    std::mem::take(&mut self.state.lock().unwrap().log)
  }

  /// Put a manifest and its blobs directly — for artifacts `janus component push` would never write.
  pub fn put(&self, repository: &str, tag: &str, manifest: &[u8], blobs: &[&[u8]]) -> String {
    let mut state = self.state.lock().unwrap();
    for blob in blobs {
      state.blobs.insert(sha256(blob), blob.to_vec());
    }
    let digest = sha256(manifest);
    let content_type = "application/vnd.oci.image.manifest.v1+json".to_string();
    for reference in [tag, &digest] {
      state.manifests.insert(
        format!("{repository}:{reference}"),
        (content_type.clone(), manifest.to_vec()),
      );
    }
    digest
  }
}

impl Drop for Registry {
  fn drop(&mut self) {
    self.server.unblock();
    if let Some(worker) = self.worker.take() {
      let _ = worker.join();
    }
  }
}
