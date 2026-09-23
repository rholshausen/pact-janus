//! A component that misbehaves on request, for the WASM loader's containment tests. It handshakes
//! honestly as a `content` component for `x/*`, then does what the decode's content type asks:
//! `x/trap` panics (a trap), `x/spin` never returns (a deadline), `x/garbage` answers bytes that are
//! not a frame. Anything else decodes to `"ok"`.

wit_bindgen::generate!({ world: "component", path: "wit" });

struct Misbehaving;

impl exports::pact::janus_component::pipe::Guest for Misbehaving {
  fn call(request: Vec<u8>) -> Vec<u8> {
    let text = String::from_utf8_lossy(&request);
    let id = field(&text, "\"id\":\"").unwrap_or_default();
    if text.contains("\"component/hello\"") {
      return format!(
        r#"{{"type":"response","id":"{id}","ok":{{"component-protocol-version":1,"component":{{"name":"misbehaving","version":"1.0.0"}},"interfaces":["content"],"contributes":{{"content-types":[{{"media-type":"x/*"}}]}}}}}}"#
      )
      .into_bytes();
    }
    #[cfg(feature = "network")]
    if text.contains("x/network") {
      let _ = std::net::TcpStream::connect("127.0.0.1:1");
    }
    if text.contains("x/trap") {
      panic!("asked to trap");
    }
    if text.contains("x/spin") {
      let mut n: u64 = 0;
      loop {
        n = std::hint::black_box(n.wrapping_add(1));
      }
    }
    if text.contains("x/garbage") {
      return b"this is not a frame".to_vec();
    }
    format!(r#"{{"type":"response","id":"{id}","ok":{{"document":"ok"}}}}"#).into_bytes()
  }
}

/// The string value after `prefix`, up to the next quote: enough JSON reading for a fixture that
/// must not depend on anything a real component would.
fn field(text: &str, prefix: &str) -> Option<String> {
  let start = text.find(prefix)? + prefix.len();
  Some(text[start..].split('"').next()?.to_string())
}

export!(Misbehaving);
