//! The text forms (spec §3) read back as tokens, so a host can style what [`render`](super::render)
//! wrote without a second renderer. It lives beside the renderer so the two change together; the
//! corpus check (`tests/plan_tokens.rs`) reads every snapshot the corpora record with it.
//!
//! Tokenizing never fails and never drops a character: the tokens' texts, joined, are the input. A
//! renderer writes a string's newlines and an error's message as they are, so the text is scanned
//! as one stream rather than line by line, and anything it does not recognise is [`Plain`], which a
//! host prints as it is.
//!
//! [`Plain`]: TokenKind::Plain

/// What a piece of a rendered plan is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
  /// `:label` or `:"a label"`.
  Container,
  /// `%namespace:name`.
  Action,
  /// `$.path` or `~>.path`.
  Resolve,
  /// `->` or `**`.
  Operator,
  /// `#{'text'}`.
  Annotation,
  /// `'a string'`.
  String,
  /// `42`, `-1.5`.
  Number,
  /// `true`, `false`, `NULL`, `BYTES(3, AQID)`.
  Literal,
  /// The executed form's `=>`.
  Arrow,
  /// `BOOL(true)`: the node succeeded.
  Pass,
  /// `BOOL(false)`: the node failed.
  Fail,
  /// `ERROR(message)`: the node failed, and says why.
  Error,
  /// Everything else: whitespace, brackets, commas.
  Plain,
}

/// A run of `text` and what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
  pub kind: TokenKind,
  pub text: &'a str,
}

/// `text`, a pretty or executed form, as tokens.
pub fn tokens(text: &str) -> Vec<Token<'_>> {
  let mut scanner = Scanner {
    text,
    at: 0,
    out: Vec::new(),
  };
  scanner.run();
  scanner.out
}

struct Scanner<'a> {
  text: &'a str,
  at: usize,
  out: Vec<Token<'a>>,
}

impl<'a> Scanner<'a> {
  fn run(&mut self) {
    let mut line_start = true;
    while self.at < self.text.len() {
      if line_start {
        line_start = false;
        self.take_while(TokenKind::Plain, |c| c == ' ');
        self.node_head();
        continue;
      }
      let rest = self.rest();
      if rest.starts_with('\n') {
        self.push(TokenKind::Plain, 1);
        line_start = true;
      } else if rest.starts_with(" => ") {
        self.push(TokenKind::Plain, 1);
        self.push(TokenKind::Arrow, 2);
        self.push(TokenKind::Plain, 1);
        self.result();
      } else {
        self.value();
      }
    }
  }

  /// What starts a node: the sigil after its indentation (spec §3.1's table). A value node's head
  /// is just a value, which the main loop reads.
  fn node_head(&mut self) {
    let rest = self.rest();
    if let Some(label) = rest.strip_prefix(':') {
      let len = match label.strip_prefix('"') {
        Some(quoted) => 1 + quoted_len(quoted, '"'),
        None => label
          .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
          .unwrap_or(label.len()),
      };
      self.push(TokenKind::Container, 1 + len);
    } else if rest.starts_with('%') {
      let name = rest.find([' ', '(', '\n']).unwrap_or(rest.len());
      self.push(TokenKind::Action, name);
    } else if rest.starts_with('$') || rest.starts_with("~>") {
      // A path is a leaf, so it is the rest of its line, up to its result or the comma after it.
      let line = rest.find('\n').unwrap_or(rest.len());
      let mut end = rest[..line].find(" => ").unwrap_or(line);
      if end == line && rest[..line].ends_with(',') {
        end -= 1;
      }
      self.push(TokenKind::Resolve, end);
    } else if rest.starts_with("->") || rest.starts_with("**") {
      self.push(TokenKind::Operator, 2);
    } else if let Some(annotation) = rest.strip_prefix("#{'") {
      let end = 3 + quoted_len(annotation, '\'');
      let end = if rest[end..].starts_with('}') {
        end + 1
      } else {
        end
      };
      self.push(TokenKind::Annotation, end);
    }
  }

  /// What follows ` => ` (spec §3.2): a verdict, an error, or a value the main loop reads.
  fn result(&mut self) {
    let rest = self.rest();
    if rest.starts_with("BOOL(true)") {
      self.push(TokenKind::Pass, "BOOL(true)".len());
    } else if rest.starts_with("BOOL(false)") {
      self.push(TokenKind::Fail, "BOOL(false)".len());
    } else if rest.starts_with("ERROR(") {
      // The message is written as it is, so it may hold anything; the error is the rest of its
      // line, but for the comma that separates it from the node after it.
      let line = rest.find('\n').unwrap_or(rest.len());
      let end = if rest[..line].ends_with("),") {
        line - 1
      } else {
        line
      };
      self.push(TokenKind::Error, end);
    }
  }

  /// One value, or one character of the punctuation between values.
  fn value(&mut self) {
    let rest = self.rest();
    let first = rest.chars().next().unwrap_or(' ');
    if first == '\'' {
      self.push(TokenKind::String, 1 + quoted_len(&rest[1..], '\''));
    } else if first.is_ascii_digit() || (first == '-' && rest[1..].starts_with(|c: char| c.is_ascii_digit()))
    {
      let end = 1
        + rest[1..]
          .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-')))
          .unwrap_or(rest.len() - 1);
      self.push(TokenKind::Number, end);
    } else if let Some(word) = ["true", "false", "NULL"].into_iter().find(|word| {
      rest.starts_with(word) && !rest[word.len()..].starts_with(|c: char| c.is_ascii_alphanumeric())
    }) {
      self.push(TokenKind::Literal, word.len());
    } else if let Some(len) = bytes_len(rest) {
      self.push(TokenKind::Literal, len);
    } else {
      self.push(TokenKind::Plain, first.len_utf8());
    }
  }

  fn rest(&self) -> &'a str {
    &self.text[self.at..]
  }

  fn take_while(&mut self, kind: TokenKind, keep: impl Fn(char) -> bool) {
    let rest = self.rest();
    let len = rest.find(|c: char| !keep(c)).unwrap_or(rest.len());
    self.push(kind, len);
  }

  /// The next `len` bytes as one token, joined to the one before when both are plain.
  fn push(&mut self, kind: TokenKind, len: usize) {
    if len == 0 {
      return;
    }
    let text = &self.text[self.at..self.at + len];
    self.at += len;
    match self.out.last_mut() {
      Some(last) if last.kind == TokenKind::Plain && kind == TokenKind::Plain => {
        let start = self.at - len - last.text.len();
        last.text = &self.text[start..self.at];
      }
      _ => self.out.push(Token { kind, text }),
    }
  }
}

/// How long a quoted run is, from just after its opening `quote` to just after its closing one
/// (or to the end, when it never closes), with `\` escaping the character after it.
fn quoted_len(rest: &str, quote: char) -> usize {
  let mut escaped = false;
  for (at, c) in rest.char_indices() {
    if escaped {
      escaped = false;
    } else if c == '\\' {
      escaped = true;
    } else if c == quote {
      return at + 1;
    }
  }
  rest.len()
}

/// How long a bytes value is when `rest` starts with one (spec §3.1): `BYTES(`, a length, `, `, base64
/// that may end in `...`, and `)`.
fn bytes_len(rest: &str) -> Option<usize> {
  let body = rest.strip_prefix("BYTES(")?;
  let (length, tail) = body.split_once(", ")?;
  if length.is_empty() || !length.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  let close = tail.find(')')?;
  let encoded = tail[..close].strip_suffix("...").unwrap_or(&tail[..close]);
  encoded
    .bytes()
    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
    .then_some("BYTES(".len() + length.len() + ", ".len() + close + 1)
}

#[cfg(test)]
mod tests {
  use super::*;
  use TokenKind::*;
  use pretty_assertions::assert_eq;

  fn kinds(text: &str) -> Vec<(TokenKind, &str)> {
    tokens(text)
      .into_iter()
      .map(|token| (token.kind, token.text))
      .collect()
  }

  #[test]
  fn the_executed_form_reads_back_as_what_each_piece_is() {
    let text = "(\n  :\"query parameters\" (\n    %expect:empty (\n      $.query => {'a': 'b'},\n      \
                #{'it\\'s'},\n      ~>,\n      -1.5 => NULL\n    ) => ERROR(Expected (none), got 1)\n  ) => BOOL(false),\n  \
                ** (\n    true\n  ) => BOOL(true)\n)";
    assert_eq!(
      kinds(text),
      vec![
        (Plain, "(\n  "),
        (Container, ":\"query parameters\""),
        (Plain, " (\n    "),
        (Action, "%expect:empty"),
        (Plain, " (\n      "),
        (Resolve, "$.query"),
        (Plain, " "),
        (Arrow, "=>"),
        (Plain, " {"),
        (String, "'a'"),
        (Plain, ": "),
        (String, "'b'"),
        (Plain, "},\n      "),
        (Annotation, "#{'it\\'s'}"),
        (Plain, ",\n      "),
        (Resolve, "~>"),
        (Plain, ",\n      "),
        (Number, "-1.5"),
        (Plain, " "),
        (Arrow, "=>"),
        (Plain, " "),
        (Literal, "NULL"),
        (Plain, "\n    ) "),
        (Arrow, "=>"),
        (Plain, " "),
        (Error, "ERROR(Expected (none), got 1)"),
        (Plain, "\n  ) "),
        (Arrow, "=>"),
        (Plain, " "),
        (Fail, "BOOL(false)"),
        (Plain, ",\n  "),
        (Operator, "**"),
        (Plain, " (\n    "),
        (Literal, "true"),
        (Plain, "\n  ) "),
        (Arrow, "=>"),
        (Plain, " "),
        (Pass, "BOOL(true)"),
        (Plain, "\n)"),
      ]
    );
  }

  #[test]
  fn a_bytes_value_is_one_literal_whole_or_cut() {
    assert_eq!(
      kinds("(\n  BYTES(4, AQIDQQ==),\n  BYTES(2048, iVBORw0K...)\n)"),
      vec![
        (Plain, "(\n  "),
        (Literal, "BYTES(4, AQIDQQ==)"),
        (Plain, ",\n  "),
        (Literal, "BYTES(2048, iVBORw0K...)"),
        (Plain, "\n)"),
      ]
    );
  }

  #[test]
  fn a_string_with_a_newline_in_it_is_one_string() {
    assert_eq!(
      kinds("(\n  'two\nlines',\n  1\n)"),
      vec![
        (Plain, "(\n  "),
        (String, "'two\nlines'"),
        (Plain, ",\n  "),
        (Number, "1"),
        (Plain, "\n)"),
      ]
    );
  }

  #[test]
  fn text_that_is_not_a_plan_is_kept_whole_and_plain() {
    let text = "not a plan at all, => ERROR(";
    let joined: std::string::String = tokens(text).iter().map(|token| token.text).collect();
    assert_eq!(joined, text);
  }
}
