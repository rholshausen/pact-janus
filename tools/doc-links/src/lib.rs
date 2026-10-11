//! The documentation link check (staged plan S0.12): does every relative link in the repository's
//! Markdown resolve?
//!
//! The ADRs cite the specifications, the specifications cite the ADRs, and both cite the prototype's
//! record — its reports under `Documentation/prototype/` and the findings each spike left under
//! `spikes/`. Those citations are the evidence trail, and a trail with a dead link in it is a claim
//! nobody can check any more. S0.12 moved a third of the documents and its exit says every link still
//! resolves; this crate is that exit as a command, so the next move (the rename S0.5 rehearses, the
//! SDKs' graduation in S2.9) is held to it too.
//!
//! What is checked: links that stay inside the repository. An inline link, a reference definition
//! or an `href`/`src` attribute whose target is a relative path must name a file or directory that
//! exists, and a `#fragment` on a Markdown target must name one of its headings, by the slug GitHub
//! gives it. A relative link that climbs out of the repository is broken by definition — it
//! resolves on the machine it was written on and nowhere else. Links with a scheme (`https:`,
//! `mailto:`) are not fetched: this check runs offline and never flakes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Directories that hold build output or dependencies, never documentation of ours.
const SKIPPED_DIRECTORIES: [&str; 4] = ["target", "node_modules", "dist", "build"];

/// A link that does not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Broken {
  /// The file the link is in, relative to the repository root.
  pub file: PathBuf,
  /// 1-based line number.
  pub line: usize,
  /// The link target as written.
  pub target: String,
  pub reason: Reason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
  /// No file or directory at the path.
  Missing,
  /// The file is there, and has no heading with the fragment's slug.
  NoSuchHeading,
  /// The path resolves to somewhere outside the repository.
  LeavesRepository,
}

impl fmt::Display for Broken {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let reason = match self.reason {
      Reason::Missing => "no such file",
      Reason::NoSuchHeading => "the file has no such heading",
      Reason::LeavesRepository => "the link leaves the repository",
    };
    write!(
      f,
      "{}:{}: {} — {reason}",
      self.file.display(),
      self.line,
      self.target
    )
  }
}

/// What a run looked at, and what it found.
#[derive(Debug, Default)]
pub struct Report {
  pub files: usize,
  pub links: usize,
  pub broken: Vec<Broken>,
}

/// Checks every Markdown file under `root`.
pub fn check(root: &Path) -> io::Result<Report> {
  let mut files = Vec::new();
  markdown_files(root, root, &mut files)?;
  files.sort();

  let mut report = Report {
    files: files.len(),
    ..Report::default()
  };
  let mut headings: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
  for file in &files {
    let text = fs::read_to_string(root.join(file))?;
    for (line, target) in links(&text) {
      report.links += 1;
      let (path, fragment) = match target.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (target.as_str(), None),
      };
      let destination = if path.is_empty() {
        Some(file.clone())
      } else {
        resolve(file.parent().unwrap_or(Path::new("")), path)
      };
      let reason = match destination {
        None => Some(Reason::LeavesRepository),
        Some(destination) if !root.join(&destination).exists() => Some(Reason::Missing),
        Some(destination) => match fragment {
          Some(fragment) if !fragment.is_empty() && is_markdown(&destination) => {
            if !headings.contains_key(&destination) {
              let slugs = heading_slugs(&fs::read_to_string(root.join(&destination))?);
              headings.insert(destination.clone(), slugs);
            }
            (!headings[&destination].contains(&fragment.to_lowercase())).then_some(Reason::NoSuchHeading)
          }
          _ => None,
        },
      };
      if let Some(reason) = reason {
        report.broken.push(Broken {
          file: file.clone(),
          line,
          target,
          reason,
        });
      }
    }
  }
  Ok(report)
}

fn markdown_files(root: &Path, directory: &Path, found: &mut Vec<PathBuf>) -> io::Result<()> {
  for entry in fs::read_dir(directory)? {
    let entry = entry?;
    let path = entry.path();
    let name = entry.file_name();
    let name = name.to_string_lossy();
    if entry.file_type()?.is_dir() {
      if !name.starts_with('.') && !SKIPPED_DIRECTORIES.contains(&name.as_ref()) {
        markdown_files(root, &path, found)?;
      }
    } else if is_markdown(&path) {
      found.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
    }
  }
  // `.github` is the one hidden directory with documentation in it.
  if directory == root && root.join(".github").is_dir() {
    markdown_files(root, &root.join(".github"), found)?;
  }
  Ok(())
}

fn is_markdown(path: &Path) -> bool {
  path.extension().is_some_and(|extension| extension == "md")
}

/// Resolves a link's `/`-separated path against the directory of the file it is in, lexically.
/// `None` if it climbs above the repository root.
fn resolve(from: &Path, link: &str) -> Option<PathBuf> {
  let mut resolved: Vec<Component> = from.components().collect();
  for part in Path::new(link).components() {
    match part {
      Component::CurDir => {}
      Component::ParentDir => {
        resolved.pop()?;
      }
      Component::Normal(_) => resolved.push(part),
      Component::RootDir | Component::Prefix(_) => return None,
    }
  }
  Some(resolved.iter().collect())
}

/// Every relative link target in a Markdown document, with its 1-based line. Fenced code blocks and
/// inline code spans are not prose, so what looks like a link inside one is not a link.
pub fn links(text: &str) -> Vec<(usize, String)> {
  let mut found = Vec::new();
  for (index, line) in prose_lines(text) {
    let mut targets = Vec::new();
    for (start, _) in line.match_indices("](") {
      if !line[..start].ends_with('\\') {
        targets.push(first_token(&line[start + 2..], ')'));
      }
    }
    for attribute in ["href=\"", "src=\""] {
      for (start, _) in line.match_indices(attribute) {
        targets.push(first_token(&line[start + attribute.len()..], '"'));
      }
    }
    // A reference definition: `[label]: target`. A footnote (`[^1]: …`) is not one.
    let trimmed = line.trim_start();
    if trimmed.starts_with('[')
      && !trimmed.starts_with("[^")
      && let Some((_, rest)) = trimmed.split_once("]:")
    {
      targets.push(first_token(rest, ' '));
    }
    for target in targets {
      if !target.is_empty() && !has_scheme(target) {
        found.push((index + 1, target.to_string()));
      }
    }
  }
  found
}

/// The link target at the start of `rest`: up to `end` or the first whitespace (a title follows
/// it), without the angle brackets Markdown allows around it.
fn first_token(rest: &str, end: char) -> &str {
  let rest = rest.trim_start();
  let stop = rest
    .find(|c: char| c == end || c.is_whitespace())
    .unwrap_or(rest.len());
  rest[..stop].trim_start_matches('<').trim_end_matches('>')
}

fn has_scheme(target: &str) -> bool {
  match target.split_once(':') {
    Some((scheme, _)) => {
      scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
          .chars()
          .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
    }
    None => false,
  }
}

/// The document's lines with fenced code blocks dropped and inline code spans blanked out.
fn prose_lines(text: &str) -> impl Iterator<Item = (usize, String)> + '_ {
  let mut fence: Option<char> = None;
  text.lines().enumerate().filter_map(move |(index, line)| {
    let trimmed = line.trim_start();
    if let Some(marker) = ['`', '~']
      .into_iter()
      .find(|marker| trimmed.starts_with(&marker.to_string().repeat(3)))
    {
      match fence {
        None => fence = Some(marker),
        Some(open) if open == marker => fence = None,
        Some(_) => {}
      }
      return None;
    }
    if fence.is_some() {
      return None;
    }
    let mut in_code = false;
    let prose = line
      .chars()
      .map(|c| {
        if c == '`' {
          in_code = !in_code;
          ' '
        } else if in_code {
          ' '
        } else {
          c
        }
      })
      .collect();
    Some((index, prose))
  })
}

/// The anchors a Markdown document offers: each heading's GitHub slug, numbered when it repeats.
pub fn heading_slugs(text: &str) -> BTreeSet<String> {
  let mut seen: BTreeMap<String, usize> = BTreeMap::new();
  let mut slugs = BTreeSet::new();
  let mut fenced = false;
  for line in text.lines() {
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
      fenced = !fenced;
      continue;
    }
    if fenced {
      continue;
    }
    let level = line.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&level) && line[level..].starts_with(' ') {
      let base = slug(line[level..].trim().trim_end_matches('#').trim_end());
      let count = seen.entry(base.clone()).or_insert(0);
      slugs.insert(if *count == 0 {
        base
      } else {
        format!("{base}-{count}")
      });
      *count += 1;
    }
  }
  slugs
}

/// GitHub's heading slug: the heading's text, lower-cased, with everything but letters, digits,
/// spaces, hyphens and underscores removed, and each space made a hyphen. A link in a heading
/// contributes its text, not its target.
pub fn slug(heading: &str) -> String {
  let mut text = String::new();
  let mut rest = heading;
  while let Some(open) = rest.find('[') {
    let Some(close) = rest[open..].find("](") else {
      break;
    };
    let Some(end) = rest[open + close..].find(')') else {
      break;
    };
    text.push_str(&rest[..open]);
    text.push_str(&rest[open + 1..open + close]);
    rest = &rest[open + close + end + 1..];
  }
  text.push_str(rest);
  text
    .trim()
    .to_lowercase()
    .chars()
    .filter(|c| c.is_alphanumeric() || " -_".contains(*c))
    .map(|c| if c == ' ' { '-' } else { c })
    .collect()
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;
  use rstest::rstest;

  use super::*;

  #[rstest]
  #[case::plain("Reassessment gates", "reassessment-gates")]
  #[case::numbered("3. The stages", "3-the-stages")]
  #[case::punctuation(
    "Stage 0a: Foundations that can start now — **S**",
    "stage-0a-foundations-that-can-start-now--s"
  )]
  #[case::code("The `janus-engine` binary", "the-janus-engine-binary")]
  #[case::link("See [ADR 0027](decisions/0027.md) first", "see-adr-0027-first")]
  #[case::underscore("pact_models and friends", "pact_models-and-friends")]
  fn a_heading_slugs_as_github_slugs_it(#[case] heading: &str, #[case] expected: &str) {
    assert_eq!(slug(heading), expected);
  }

  #[test]
  fn a_repeated_heading_is_numbered() {
    let slugs = heading_slugs("# Decision\n\n## Context\n\n```\n# not a heading\n```\n\n## Context\n");
    assert_eq!(
      slugs.into_iter().collect::<Vec<_>>(),
      vec![
        "context".to_string(),
        "context-1".to_string(),
        "decision".to_string()
      ]
    );
  }

  #[test]
  fn links_are_found_in_prose_and_nowhere_else() {
    let text = "\
See [the plan](../plan.md#stages) and [the RFC](https://example.org/rfc), or `[not](a-link.md)`.
<img src=\"./logo.svg\" width=\"120\" /> and <a href=\"mailto:someone@example.org\">mail</a>
```sh
echo \"[nor](this.md)\"
```
[gate]: prototype/project-plan.md \"the gates\"
[^1]: a footnote, [with](a-link.md) in it
An escaped \\](bracket.md) and a [titled](<spaced.md> \"title\") link.
";
    assert_eq!(
      links(text),
      vec![
        (1, "../plan.md#stages".to_string()),
        (2, "./logo.svg".to_string()),
        (6, "prototype/project-plan.md".to_string()),
        (7, "a-link.md".to_string()),
        (8, "spaced.md".to_string()),
      ]
    );
  }

  #[rstest]
  #[case::sibling("Documentation/decisions", "0027.md", Some("Documentation/decisions/0027.md"))]
  #[case::up(
    "Documentation/decisions",
    "../prototype/project-plan.md",
    Some("Documentation/prototype/project-plan.md")
  )]
  #[case::to_the_root("Documentation", "../README.md", Some("README.md"))]
  #[case::out("Documentation/prototype", "../../../roadmap/rfc.md", None)]
  #[case::absolute("Documentation", "/etc/hosts", None)]
  fn a_link_resolves_against_its_file(
    #[case] from: &str,
    #[case] link: &str,
    #[case] expected: Option<&str>,
  ) {
    assert_eq!(resolve(Path::new(from), link), expected.map(PathBuf::from));
  }
}
