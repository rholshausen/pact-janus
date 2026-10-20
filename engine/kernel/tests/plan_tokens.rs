//! Every text form the corpora record (`corpora/**/plan.txt` and `executed.txt`) reads back as
//! tokens with nothing lost and nothing left over: joined, the tokens are the snapshot, and the only
//! text no token names is whitespace and punctuation. A renderer change the tokenizer cannot read
//! fails here, beside the snapshot that shows it.

use pact_janus_kernel::plan::{TokenKind, tokens};
use std::path::{Path, PathBuf};

fn snapshots(dir: &Path, found: &mut Vec<PathBuf>) {
  for entry in std::fs::read_dir(dir).expect("a corpus directory") {
    let path = entry.expect("a directory entry").path();
    if path.is_dir() {
      snapshots(&path, found);
    } else if path
      .file_name()
      .is_some_and(|name| name == "plan.txt" || name == "executed.txt")
    {
      found.push(path);
    }
  }
}

#[test]
fn every_text_form_the_corpora_record_reads_back_as_tokens() {
  let corpora = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpora");
  let mut found = Vec::new();
  snapshots(&corpora, &mut found);
  assert!(
    found.len() > 20,
    "found only {} snapshots under {}",
    found.len(),
    corpora.display()
  );

  for path in found {
    let text = std::fs::read_to_string(&path).expect("a snapshot");
    let read = tokens(&text);
    let joined: String = read.iter().map(|token| token.text).collect();
    assert_eq!(joined, text, "{}: the tokens are not the text", path.display());
    for token in read.iter().filter(|token| token.kind == TokenKind::Plain) {
      assert!(
        token
          .text
          .chars()
          .all(|c| c.is_whitespace() || "()[]{},:".contains(c)),
        "{}: no token names {:?}",
        path.display(),
        token.text
      );
    }
  }
}
