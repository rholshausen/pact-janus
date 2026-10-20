//! ADR 0035 decision 2: building a client and sending through it starts no thread. A request runs
//! on the calling thread, its name lookup included, which reqwest's default resolver would hand to a
//! thread of its own.
//!
//! The one test in this file, so nothing else runs beside it while it counts the process's threads.
//! The count is read from `/proc`, so it runs on Linux; the client is the same code on every system.

#![cfg(target_os = "linux")]

use pact_janus_http_client::{Defaults, HttpClient};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn threads() -> usize {
  std::fs::read_dir("/proc/self/task")
    .expect("Linux lists a process's threads")
    .count()
}

#[test]
fn a_request_starts_no_thread() {
  // The provider's thread is started before counting, and answers on it.
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let port = listener.local_addr().unwrap().port();
  thread::spawn(move || {
    for mut stream in listener.incoming().flatten() {
      let mut buffer = [0u8; 4096];
      let _ = stream.read(&mut buffer);
      let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
    }
  });
  // Let the provider's thread start before it is counted.
  thread::sleep(Duration::from_millis(50));
  let before = threads();

  let client = HttpClient::from_value(
    None,
    Defaults {
      request_timeout: Duration::from_secs(5),
      retries: 0,
      user_agent: "pact-janus-tests".to_string(),
    },
  )
  .expect("the defaults build");
  for _ in 0..3 {
    let request = http::Request::get(format!("http://localhost:{port}/"))
      .body(Vec::new())
      .unwrap();
    let response = client.run(request, None).expect("the provider answers");
    assert_eq!(response.body(), b"ok");
  }

  assert_eq!(threads(), before, "the client started a thread");
}
