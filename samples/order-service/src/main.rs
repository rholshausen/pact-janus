//! `order-service`: the sample provider as a process, for demos and for a verification run that
//! wants a target outside its own test binary. Flags are parsed by hand — a sample whose
//! dependency list is longer than its behaviour would be telling the wrong story.

use pact_janus_sample_order_service::{Config, DEFAULT_TOKEN, start};
use std::process::ExitCode;

fn main() -> ExitCode {
  let mut config = Config::default();
  let mut args = std::env::args().skip(1);
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--host" => config.host = args.next().unwrap_or_else(|| config.host.clone()),
      "--port" => config.port = args.next().and_then(|p| p.parse().ok()).unwrap_or(0),
      "--token" => config.token = args.next(),
      "--no-auth" => config.token = None,
      "--help" | "-h" => {
        usage();
        return ExitCode::SUCCESS;
      }
      other => {
        eprintln!("unknown argument '{other}'\n");
        usage();
        return ExitCode::from(2);
      }
    }
  }

  let provider = match start(config.clone()) {
    Ok(provider) => provider,
    Err(err) => {
      eprintln!("{err}");
      return ExitCode::FAILURE;
    }
  };

  // stdout carries the base URL and nothing else, so a script can read it directly.
  println!("{}", provider.base_url());
  match &config.token {
    Some(token) => eprintln!("bearer token: {token}"),
    None => eprintln!("auth disabled"),
  }
  eprintln!(
    "provider states: POST {}/_pact/provider-states",
    provider.base_url()
  );
  eprintln!("press ctrl-c to stop");

  // The provider runs on its own thread; this one has nothing to do but stay alive.
  loop {
    std::thread::park();
  }
}

fn usage() {
  eprintln!("order-service — the Pact Janus sample provider (plan task 5.6)");
  eprintln!();
  eprintln!("  --host <host>    interface to bind (default 127.0.0.1)");
  eprintln!("  --port <port>    port to bind, 0 for an OS-assigned one (default 0)");
  eprintln!("  --token <token>  the bearer token to require (default {DEFAULT_TOKEN})");
  eprintln!("  --no-auth        accept requests with no Authorization header");
}
