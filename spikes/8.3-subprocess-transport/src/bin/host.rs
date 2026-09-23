//! A host process for the orphan test: loads one subprocess component, prints the pid its
//! handshake reports, and waits to be killed. What happens to the component when it is — with no
//! chance to clean up — is the test (spec §9.3; spike 1.3 finding 2).

use pact_janus_kernel::component::{ComponentDeclaration, ComponentLoader, Grants, Limits, Source};
use spike_subprocess_transport::SubprocessLoader;
use std::io::Write;

fn main() {
  let command = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
  let loaded = SubprocessLoader
    .load(&ComponentDeclaration {
      name: "misbehaving".to_string(),
      source: Source {
        kind: "subprocess".to_string(),
        reference: Some(command),
        digest: None,
      },
      grants: Grants::default(),
      limits: Limits::default(),
    })
    .unwrap_or_else(|err| panic!("{err:?}"));
  println!("{}", loaded.hello["pid"]);
  std::io::stdout().flush().unwrap();
  loop {
    std::thread::sleep(std::time::Duration::from_secs(60));
  }
}
