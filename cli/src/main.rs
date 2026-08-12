//! The `pact` CLI. Subcommands (verify, explain, upgrade, check) arrive with their phases;
//! until then this only reports versions, which also proves the workspace wiring.

fn main() {
  println!(
    "pact-janus {} (engine protocol v{})",
    pact_janus_kernel::ENGINE_VERSION,
    pact_janus_kernel::PROTOCOL_VERSION
  );
}
