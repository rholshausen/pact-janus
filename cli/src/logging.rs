//! Where `janus` and `janus-engine` send their logs: stderr, so stdout stays the command's answer
//! (or, for `janus-engine`, frames only). Both binaries share one subscriber and differ only in the
//! filter they start from.

use tracing::Subscriber;
use tracing_subscriber::filter::{EnvFilter, FilterExt, LevelFilter, Targets};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::{Filter, Layer as _, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

/// Installs the subscriber for this process: `filter`'s events on stderr, without credentials.
pub fn to_stderr(filter: EnvFilter) {
  subscriber(filter, std::io::stderr, stderr_supports_colour()).init();
}

fn subscriber<W>(filter: EnvFilter, writer: W, ansi: bool) -> impl Subscriber + Send + Sync
where
  W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
  tracing_subscriber::registry().with(
    tracing_subscriber::fmt::layer()
      .with_writer(writer)
      .with_ansi(ansi)
      .with_filter(without_credentials(filter)),
  )
}

/// `filter`, with what would log a credential held back however much was asked for: at `trace`,
/// an HTTP library may dump a request's bytes, a broker's `Authorization` header or a provider's
/// custom headers among them (engine-protocol spec §8.7: a credential is never repeated in a log).
/// The engine's own client (ADR 0035) is reqwest over hyper, h2 and rustls; the hooks' `http`
/// implementation and the tests' own clients are ureq. Their `debug` is kept, and says nothing
/// secret.
///
/// The cap is a second filter an event must also pass, not a directive in `filter`: a directive
/// sets the target's level, so `ureq_proto=debug` would log its `debug` at `warn` too.
fn without_credentials<S>(filter: EnvFilter) -> impl Filter<S>
where
  S: Subscriber + for<'a> LookupSpan<'a>,
{
  filter.and(
    Targets::new()
      .with_default(LevelFilter::TRACE)
      .with_target("ureq_proto", LevelFilter::DEBUG)
      .with_target("reqwest", LevelFilter::DEBUG)
      .with_target("hyper", LevelFilter::DEBUG)
      .with_target("hyper_util", LevelFilter::DEBUG)
      .with_target("h2", LevelFilter::DEBUG)
      .with_target("rustls", LevelFilter::DEBUG),
  )
}

/// Whether logs on stderr may be coloured: only on a terminal, and only while `NO_COLOR` is unset
/// (no-color.org).
fn stderr_supports_colour() -> bool {
  use std::io::IsTerminal;
  std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use std::sync::{Arc, Mutex};

  #[derive(Clone, Default)]
  struct Captured(Arc<Mutex<Vec<u8>>>);

  impl std::io::Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
      self.0.lock().expect("the buffer").extend_from_slice(bytes);
      Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
      Ok(())
    }
  }

  impl<'w> MakeWriter<'w> for Captured {
    type Writer = Captured;

    fn make_writer(&'w self) -> Self::Writer {
      self.clone()
    }
  }

  /// Which of a fixed set of events `filter` lets through.
  fn logged(filter: &str) -> Vec<&'static str> {
    let captured = Captured::default();
    tracing::subscriber::with_default(
      subscriber(EnvFilter::new(filter), captured.clone(), false),
      || {
        tracing::trace!(target: "ureq_proto::client", "http-trace");
        tracing::debug!(target: "ureq_proto::client", "http-debug");
        tracing::warn!(target: "ureq_proto::client", "http-warn");
        tracing::debug!(target: "pact_janus_kernel", "engine-debug");
        tracing::warn!(target: "pact_janus_kernel", "engine-warn");
      },
    );
    let text = String::from_utf8(captured.0.lock().expect("the buffer").clone()).expect("utf-8");
    [
      "http-trace",
      "http-debug",
      "http-warn",
      "engine-debug",
      "engine-warn",
    ]
    .into_iter()
    .filter(|message| text.contains(message))
    .collect()
  }

  #[rstest::rstest]
  #[case::warn("warn", vec!["http-warn", "engine-warn"])]
  #[case::debug("debug", vec!["http-debug", "http-warn", "engine-debug", "engine-warn"])]
  #[case::trace("trace", vec!["http-debug", "http-warn", "engine-debug", "engine-warn"])]
  #[case::asked_for_by_name("warn,ureq_proto=trace", vec!["http-debug", "http-warn", "engine-warn"])]
  fn the_http_library_logs_at_the_level_asked_for_and_never_at_trace(
    #[case] filter: &str,
    #[case] expected: Vec<&str>,
  ) {
    assert_eq!(logged(filter), expected);
  }

  /// The engine's own client (ADR 0035) is held to `debug` too, at every layer beneath it.
  #[test]
  fn the_engines_http_client_never_logs_at_trace() {
    for target in [
      "reqwest::connect",
      "hyper::proto::h1::io",
      "hyper_util::client::legacy::pool",
      "h2::codec::framed_write",
      "rustls::client::hs",
    ] {
      let captured = Captured::default();
      tracing::subscriber::with_default(
        subscriber(EnvFilter::new("trace"), captured.clone(), false),
        || {
          // A target is a literal to `tracing`, so each is spelled out.
          match target {
            "reqwest::connect" => tracing::trace!(target: "reqwest::connect", "client-trace"),
            "hyper::proto::h1::io" => tracing::trace!(target: "hyper::proto::h1::io", "client-trace"),
            "hyper_util::client::legacy::pool" => {
              tracing::trace!(target: "hyper_util::client::legacy::pool", "client-trace")
            }
            "h2::codec::framed_write" => tracing::trace!(target: "h2::codec::framed_write", "client-trace"),
            _ => tracing::trace!(target: "rustls::client::hs", "client-trace"),
          }
          tracing::debug!(target: "hyper::proto::h1::io", "client-debug");
        },
      );
      let text = String::from_utf8(captured.0.lock().expect("the buffer").clone()).expect("utf-8");
      assert!(!text.contains("client-trace"), "{target}: {text}");
      assert!(text.contains("client-debug"), "{target}: {text}");
    }
  }
}
