//! A very small argument parser (plan task 5.5: "real flags, a parser").
//!
//! Hand-rolled rather than pulled in, for the reason the TypeScript conventions give about SDK
//! dependencies and which applies just as well here: this repository is a prototype whose point is
//! that the *engine* is the product, and every dependency is part of the story it tells. Three
//! subcommands with a dozen flags between them do not need a parser framework, and one that reads
//! in a page is one a reviewer can finish.
//!
//! The grammar is deliberately boring: `--name value` and `--flag`, `--` ends option parsing, and
//! anything else is a positional. No abbreviations, no clustering, no `=` form — a CLI that
//! accepts one spelling of each option can never be ambiguous about what a user meant.

use std::collections::BTreeMap;

pub struct Args {
  positionals: Vec<String>,
  options: BTreeMap<String, Vec<String>>,
  flags: Vec<String>,
}

/// What an option takes, so the parser can tell `--config path` from `--explain-failures`.
pub struct Spec {
  /// Options that take exactly one value, and may be repeated.
  pub values: &'static [&'static str],
  /// Options that take none.
  pub flags: &'static [&'static str],
}

impl Args {
  /// Parse `argv` (already stripped of the program name and the subcommand).
  pub fn parse(argv: &[String], spec: &Spec) -> Result<Args, String> {
    let mut args = Args {
      positionals: Vec::new(),
      options: BTreeMap::new(),
      flags: Vec::new(),
    };
    let mut rest = argv.iter();
    let mut only_positionals = false;
    while let Some(arg) = rest.next() {
      if only_positionals || !arg.starts_with("--") {
        args.positionals.push(arg.clone());
        continue;
      }
      if arg == "--" {
        only_positionals = true;
        continue;
      }
      let name = arg.trim_start_matches("--");
      if spec.flags.contains(&name) {
        args.flags.push(name.to_string());
      } else if spec.values.contains(&name) {
        let value = rest
          .next()
          .ok_or_else(|| format!("'{arg}' needs a value"))?
          .clone();
        args.options.entry(name.to_string()).or_default().push(value);
      } else {
        return Err(format!("unknown option '{arg}'"));
      }
    }
    Ok(args)
  }

  pub fn positionals(&self) -> &[String] {
    &self.positionals
  }

  pub fn value(&self, name: &str) -> Option<&str> {
    self.options.get(name).and_then(|v| v.last()).map(String::as_str)
  }

  pub fn values(&self, name: &str) -> Vec<String> {
    self.options.get(name).cloned().unwrap_or_default()
  }

  pub fn flag(&self, name: &str) -> bool {
    self.flags.iter().any(|f| f == name)
  }
}
