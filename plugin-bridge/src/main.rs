//! `janus-plugin-bridge <plugin> <version>`: one of today's pact-plugins, served to the Janus engine as a
//! component over the subprocess binding (component-interfaces spec §9.3, ADR 0106, ADR 0109).
//!
//! The engine spawns it from a `subprocess` declaration, one process per plugin, named
//! `pact-plugin-<plugin>`:
//!
//! ```yaml
//! components:
//!   - name: pact-plugin-protobuf
//!     source: { kind: subprocess, reference: janus-plugin-bridge, args: [protobuf, 0.8.1] }
//!     grants: { env: [HOME, PACT_PLUGIN_DIR] }
//! ```
//!
//! stdout carries frames and nothing else; logs go to stderr, at the level `RUST_LOG` names (`warn`
//! unless it is granted and set). When stdin ends, the plugin is shut down and the bridge exits; when the
//! plugin cannot be reached, the bridge answers that call as `unavailable` and exits too.

mod bridge;
mod catalogue;
mod compare;
mod error;
mod frames;
mod generate;

use bridge::{Bridge, Wanted};
use std::io::{self, BufReader};
use std::process::ExitCode;

const USAGE: &str = "usage: janus-plugin-bridge <plugin-name> <plugin-version>

Serves one pact plugin to the Janus engine as the component 'pact-plugin-<plugin-name>', speaking
component frames on stdin and stdout (component-interfaces spec §9.3). The engine starts it from a
'subprocess' declaration; it is not meant to be run by hand.";

fn main() -> ExitCode {
  let args: Vec<String> = std::env::args().skip(1).collect();
  match args.first().map(String::as_str) {
    Some("--version" | "-V") => {
      println!("janus-plugin-bridge {}", env!("CARGO_PKG_VERSION"));
      return ExitCode::SUCCESS;
    }
    Some("--help" | "-h") => {
      println!("{USAGE}");
      return ExitCode::SUCCESS;
    }
    _ => {}
  }
  // stdout is the frame channel from here on: nothing but `frames::write` may reach it.
  tracing_subscriber::fmt()
    .with_writer(io::stderr)
    .with_ansi(false)
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
    )
    .init();

  let wanted = wanted(&args);
  if let Err(why) = &wanted {
    tracing::error!("{why}");
  }
  let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
    Ok(runtime) => runtime,
    Err(err) => {
      eprintln!("janus-plugin-bridge: could not start its runtime: {err}");
      return ExitCode::from(2);
    }
  };
  let mut bridge = Bridge::new(wanted, runtime);
  let mut stdin = BufReader::new(io::stdin().lock());
  let mut stdout = io::stdout().lock();
  let code = loop {
    match frames::read(&mut stdin) {
      Ok(Some(bytes)) => {
        let response = bridge.answer(&bytes);
        if let Err(err) = frames::write(&mut stdout, &response) {
          tracing::error!("could not write a frame to stdout: {err}");
          break ExitCode::FAILURE;
        }
        // The plugin is gone, and this process is what the engine recreates (ADR 0108 decision 3):
        // leaving, once the answer is written, gets the next call a new bridge and a new plugin.
        if bridge.gone() {
          break ExitCode::FAILURE;
        }
      }
      Ok(None) => break ExitCode::SUCCESS,
      Err(err) => {
        tracing::error!("could not read a frame from stdin: {err}");
        break ExitCode::FAILURE;
      }
    }
  };
  bridge.shutdown();
  code
}

/// The plugin the command line names, or why it names none a component can be made of.
fn wanted(args: &[String]) -> Result<Wanted, String> {
  match args {
    [name, version] => {
      if catalogue::usable_name(name) {
        Ok(Wanted {
          name: name.clone(),
          version: version.clone(),
        })
      } else {
        Err(format!(
          "the pact plugin '{name}' cannot be served as a component: a component's name is lower-case letters, digits and '-' (component-interfaces spec §2.1)"
        ))
      }
    }
    _ => Err(format!(
      "janus-plugin-bridge was started with {} argument(s), and takes two: the plugin's name and its version",
      args.len()
    )),
  }
}
