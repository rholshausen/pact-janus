//! The repository's own documentation, held to the check: S0.12's exit, "every link in
//! `Documentation/` resolves", and the same for every other Markdown file that cites it.

use std::path::Path;

use pact_janus_doc_links::check;

#[test]
fn every_relative_link_in_the_repository_resolves() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .expect("the crate sits two levels below the repository root");

  let report = check(root).expect("the repository's Markdown is readable");

  assert!(report.files > 0, "found no Markdown under {}", root.display());
  let broken: Vec<String> = report.broken.iter().map(ToString::to_string).collect();
  assert!(
    broken.is_empty(),
    "links that do not resolve:\n{}",
    broken.join("\n")
  );
}
