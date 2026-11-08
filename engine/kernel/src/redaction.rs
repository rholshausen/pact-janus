//! Credentials kept out of what the engine records and reports (ADR 0074, staged plan S2.8).
//!
//! A **redacted member** is a path: a part, a slot, then member names (`request.headers.authorization`).
//! Which members carry credentials is the transport's to say ([`CredentialMembers`]), so nothing here
//! names a header: the kernel walks documents by the names it is given.
//!
//! Two things are done with a set of members. A contract an engine writes has each member's values and
//! examples replaced ([`redact_interaction`]). And everything a run reports has every string found under
//! a member replaced wherever it appears ([`Secrets`]): a value is a secret wherever it ends up, and a
//! mismatch message quotes the value, not the member it came from.

use crate::contract::{self, SlotValue};
use crate::display::Marked;
use crate::plan::{Executed, ExecutedKind, Mismatch, NodeResult, RuntimeValue};
use base64::Engine as _;
use serde_json::Value;
use std::collections::BTreeMap;

/// What a redacted value is replaced by, as a string (ADR 0074 decision 4).
pub const PLACEHOLDER: &str = "[redacted]";

/// Which members of which slots carry credentials, as a transport declares them (component-interfaces
/// spec §5.8): part name -> slot name -> member names.
pub type CredentialMembers = BTreeMap<String, BTreeMap<String, Vec<String>>>;

/// One redacted member (ADR 0074 decision 1). An empty `path` is the whole slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
  pub part: String,
  pub slot: String,
  pub path: Vec<String>,
}

impl Member {
  /// Read a path as a plan path is spelled without its `$.`: names joined by `.`, or quoted as
  /// `['name']` when a name has a character the plain form would end at.
  pub fn parse(text: &str) -> Result<Member, String> {
    let mut names = Vec::new();
    let mut rest = text;
    let mut first = true;
    while !rest.is_empty() || first {
      let after = if first {
        rest
      } else if let Some(after) = rest.strip_prefix('.') {
        after
      } else if rest.starts_with("['") {
        rest
      } else {
        return Err(format!(
          "'{text}' is not a path: expected '.' or \"['\" at '{rest}'"
        ));
      };
      first = false;
      if let Some(quoted) = after.strip_prefix("['") {
        let (name, remaining) = crate::plan::unquote(quoted);
        if !quoted[..quoted.len() - remaining.len()].ends_with("']") {
          return Err(format!("'{text}' is not a path: a quoted name is not closed"));
        }
        names.push(name);
        rest = remaining;
      } else {
        let end = after.find(['.', '[']).unwrap_or(after.len());
        let (name, remaining) = after.split_at(end);
        if name.is_empty() {
          return Err(format!("'{text}' is not a path: it has an empty name"));
        }
        if remaining.starts_with('[') && !remaining.starts_with("['") {
          return Err(format!(
            "'{text}' is not a path: it names members, not elements, so it has no index"
          ));
        }
        names.push(name.to_string());
        rest = remaining;
      }
    }
    if names.len() < 2 {
      return Err(format!(
        "'{text}' is not a path: it names a part and a slot at least"
      ));
    }
    let mut names = names.into_iter();
    let part = names.next().expect("checked above");
    let slot = names.next().expect("checked above");
    Ok(Member {
      part,
      slot,
      path: names.collect(),
    })
  }

  /// The path as [`Member::parse`] reads it.
  pub fn to_path(&self) -> String {
    let part = crate::plan::member_path("", &self.part);
    let mut path = part.strip_prefix('.').map(str::to_string).unwrap_or(part);
    path = crate::plan::member_path(&path, &self.slot);
    for name in &self.path {
      path = crate::plan::member_path(&path, name);
    }
    path
  }

  /// Whether two paths name the same member, ignoring ASCII case (ADR 0074 decision 1).
  fn same(&self, other: &Member) -> bool {
    self.part.eq_ignore_ascii_case(&other.part)
      && self.slot.eq_ignore_ascii_case(&other.slot)
      && self.path.len() == other.path.len()
      && self
        .path
        .iter()
        .zip(&other.path)
        .all(|(a, b)| a.eq_ignore_ascii_case(b))
  }

  /// Every name, part and slot first.
  fn names(&self) -> impl Iterator<Item = &str> {
    [self.part.as_str(), self.slot.as_str()]
      .into_iter()
      .chain(self.path.iter().map(String::as_str))
  }
}

/// A transport's declared members as paths.
pub fn members_of(declared: &CredentialMembers) -> Vec<Member> {
  declared
    .iter()
    .flat_map(|(part, slots)| {
      slots.iter().flat_map(move |(slot, names)| {
        names.iter().map(move |name| Member {
          part: part.clone(),
          slot: slot.clone(),
          path: vec![name.clone()],
        })
      })
    })
    .collect()
}

/// `members` with each member named once.
fn unique(members: impl IntoIterator<Item = Member>) -> Vec<Member> {
  let mut unique: Vec<Member> = Vec::new();
  for member in members {
    if !unique.iter().any(|seen| seen.same(&member)) {
      unique.push(member);
    }
  }
  unique
}

/// A consumer session's `config.redaction` (ADR 0074 decision 3).
#[derive(Debug, Clone, Default)]
pub struct SessionRedaction {
  add: Vec<Member>,
  keep: Vec<Member>,
}

impl SessionRedaction {
  /// Read `{ add, keep }`, each a list of paths. A path that does not parse is a problem at its
  /// pointer under `at`.
  pub fn parse(document: Option<&Value>, at: &str) -> Result<SessionRedaction, Vec<contract::Problem>> {
    let Some(document) = document else {
      return Ok(SessionRedaction::default());
    };
    let mut problems = Vec::new();
    let mut read = |name: &str| -> Vec<Member> {
      let Some(listed) = document.get(name) else {
        return Vec::new();
      };
      let Some(items) = listed.as_array() else {
        problems.push(contract::Problem {
          pointer: format!("{at}/{name}"),
          message: "is not a list of paths".to_string(),
        });
        return Vec::new();
      };
      items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
          let parsed = item
            .as_str()
            .ok_or_else(|| "is not a path".to_string())
            .and_then(Member::parse);
          parsed
            .map_err(|message| {
              problems.push(contract::Problem {
                pointer: format!("{at}/{name}/{index}"),
                message,
              })
            })
            .ok()
        })
        .collect()
    };
    let add = read("add");
    let keep = read("keep");
    if problems.is_empty() {
      Ok(SessionRedaction { add, keep })
    } else {
      Err(problems)
    }
  }

  /// What a contract records redacted: the transport's members and `add`, less `keep`.
  pub fn recorded(&self, defaults: &[Member]) -> Vec<Member> {
    unique(defaults.iter().chain(&self.add).cloned())
      .into_iter()
      .filter(|member| !self.keep.iter().any(|kept| kept.same(member)))
      .collect()
  }

  /// What a report never repeats: the transport's members and `add`. `keep` is about recording.
  pub fn reported(&self, defaults: &[Member]) -> Vec<Member> {
    unique(defaults.iter().chain(&self.add).cloned())
  }
}

/// Each transport kind's declared members, and a session's own choices (ADR 0074 decisions 2 and 3).
#[derive(Debug, Clone, Default)]
pub struct Redaction {
  defaults: BTreeMap<String, Vec<Member>>,
  session: SessionRedaction,
}

impl Redaction {
  pub fn new(session: SessionRedaction) -> Redaction {
    Redaction {
      defaults: BTreeMap::new(),
      session,
    }
  }

  /// What a transport of `kind` declares.
  pub fn with_transport(
    mut self,
    kind: &str,
    transport: &dyn crate::component::TransportComponent,
  ) -> Redaction {
    self
      .defaults
      .insert(kind.to_string(), members_of(&transport.credential_members()));
    self
  }

  /// The members the transport an interaction of `kind` goes through declares; an interaction that
  /// names none goes through the default kind's.
  pub fn defaults(&self, kind: Option<&str>) -> &[Member] {
    self
      .defaults
      .get(kind.unwrap_or(crate::common::DEFAULT_TRANSPORT_KIND))
      .map(Vec::as_slice)
      .unwrap_or_default()
  }

  /// What a contract records redacted for an interaction of `kind`.
  pub fn recorded(&self, kind: Option<&str>) -> Vec<Member> {
    self.session.recorded(self.defaults(kind))
  }

  /// What nothing reported repeats for an interaction of `kind`.
  pub fn reported(&self, kind: Option<&str>) -> Vec<Member> {
    self.session.reported(self.defaults(kind))
  }
}

/// A contract's own `redacted` paths as members, skipping any this engine cannot read.
pub fn recorded_members(interaction: &contract::Interaction) -> Vec<Member> {
  interaction
    .redacted
    .iter()
    .flatten()
    .filter_map(|path| Member::parse(path).ok())
    .collect()
}

/// `defaults` and `more`, each named once.
pub fn union(defaults: &[Member], more: &[Member]) -> Vec<Member> {
  unique(defaults.iter().chain(more).cloned())
}

// --- Recording -------------------------------------------------------------------------------------

/// Redact the members of one interaction a contract records (ADR 0074 decision 4), and list the ones
/// it found in `redacted`. Answers the paths redacted this time, in order.
pub fn redact_interaction(interaction: &mut contract::Interaction, members: &[Member]) -> Vec<String> {
  let mut redacted = Vec::new();
  for member in unique(members.iter().cloned()) {
    let mut found = false;
    for (part, slots) in interaction.parts.iter_mut() {
      if !part.eq_ignore_ascii_case(&member.part) {
        continue;
      }
      for (slot, shape) in slots.iter_mut() {
        if slot.eq_ignore_ascii_case(&member.slot) {
          found |= redact_shape_at(shape, &member.path);
        }
      }
    }
    for variant in interaction.selection.variants.iter_mut() {
      for (part, slots) in variant.parts.iter_mut() {
        if !part.eq_ignore_ascii_case(&member.part) {
          continue;
        }
        for (slot, value) in slots.iter_mut() {
          if slot.eq_ignore_ascii_case(&member.slot) {
            found |= redact_slot_value(value, &member.path);
          }
        }
      }
    }
    if found {
      redacted.push(member.to_path());
    }
  }
  if !redacted.is_empty() {
    let mut listed = interaction.redacted.take().unwrap_or_default();
    for path in &redacted {
      if !listed.contains(path) {
        listed.push(path.clone());
      }
    }
    listed.sort();
    interaction.redacted = Some(listed);
  }
  redacted
}

/// A shape node is an object naming its operator.
fn is_shape(value: &Value) -> bool {
  value.get("shape").is_some_and(Value::is_string)
}

/// The members of a node holding nodes it is made of, other than `members` (which a name steps into)
/// and the members that hold values (`example`, `generator`): `of`, `items`, `entries`, the
/// alternatives of a choice. Read generically, so an operator added later is walked too.
fn child_nodes(node: &mut Value) -> Vec<&mut Value> {
  let Value::Object(members) = node else {
    return Vec::new();
  };
  let mut children = Vec::new();
  for (name, member) in members.iter_mut() {
    if matches!(name.as_str(), "shape" | "example" | "generator" | "members") {
      continue;
    }
    match member {
      child if is_shape(child) => children.push(child),
      Value::Array(items) if items.iter().all(is_shape) => children.extend(items.iter_mut()),
      _ => {}
    }
  }
  children
}

/// [`child_nodes`], read only.
fn child_nodes_of(node: &Value) -> Vec<&Value> {
  let Value::Object(members) = node else {
    return Vec::new();
  };
  let mut children = Vec::new();
  for (name, member) in members {
    if matches!(name.as_str(), "shape" | "example" | "generator" | "members") {
      continue;
    }
    match member {
      child if is_shape(child) => children.push(child),
      Value::Array(items) if items.iter().all(is_shape) => children.extend(items.iter()),
      _ => {}
    }
  }
  children
}

/// Step into the nodes `path` names under `node` and redact each; true when one was found.
fn redact_shape_at(node: &mut Value, path: &[String]) -> bool {
  if !is_shape(node) {
    return false;
  }
  let Some((name, rest)) = path.split_first() else {
    redact_shape(node);
    return true;
  };
  let mut found = false;
  if let Some(Value::Object(members)) = node.get_mut("members") {
    for (member, child) in members.iter_mut() {
      if member.eq_ignore_ascii_case(name) {
        found |= redact_shape_at(child, rest);
      }
    }
  }
  for child in child_nodes(node) {
    found |= redact_shape_at(child, path);
  }
  found
}

/// Redact a node and everything under it: `equality` becomes `type`, and every example its
/// placeholder.
fn redact_shape(node: &mut Value) {
  let encoded = node.get("encoded").and_then(Value::as_str).map(str::to_string);
  if let Value::Object(members) = node {
    if members.get("shape").and_then(Value::as_str) == Some("equality") {
      members.insert("shape".to_string(), Value::String("type".to_string()));
    }
    if let Some(example) = members.get_mut("example") {
      *example = placeholder_of(example, encoded.as_deref());
    }
    if let Some(Value::Object(named)) = members.get_mut("members") {
      named.values_mut().for_each(redact_shape);
    }
  }
  for child in child_nodes(node) {
    redact_shape(child);
  }
}

/// A value's placeholder, of its kind and structure (ADR 0074 decision 4). A string the `encoded` tag
/// says is base64 becomes the placeholder's base64.
fn placeholder_of(value: &Value, encoded: Option<&str>) -> Value {
  match value {
    Value::String(_) if encoded == Some("base64") => {
      Value::String(base64::engine::general_purpose::STANDARD.encode(PLACEHOLDER))
    }
    Value::String(_) => Value::String(PLACEHOLDER.to_string()),
    Value::Number(_) => Value::from(0),
    Value::Bool(_) => Value::Bool(false),
    Value::Null => Value::Null,
    Value::Array(items) => Value::Array(items.iter().map(|item| placeholder_of(item, None)).collect()),
    Value::Object(members) => Value::Object(
      members
        .iter()
        .map(|(name, member)| (name.clone(), placeholder_of(member, None)))
        .collect(),
    ),
  }
}

/// Redact a recorded slot value at `path`; true when there was something there. A slot recorded as
/// octets has no members to find, so a path into one replaces the whole value.
fn redact_slot_value(slot: &mut SlotValue, path: &[String]) -> bool {
  match slot.encoded.as_deref() {
    Some(encoded @ ("text" | "base64")) => {
      slot.content = placeholder_of(&Value::String(String::new()), Some(encoded));
      true
    }
    _ if path.is_empty() => {
      slot.content = placeholder_of(&slot.content, None);
      true
    }
    _ => redact_value_at(&mut slot.content, path),
  }
}

/// Replace what `path` names in a document; true when it named something.
fn redact_value_at(value: &mut Value, path: &[String]) -> bool {
  let Some((name, rest)) = path.split_first() else {
    *value = placeholder_of(value, None);
    return true;
  };
  match value {
    Value::Object(members) => {
      let mut found = false;
      for (member, child) in members.iter_mut() {
        if member.eq_ignore_ascii_case(name) {
          found |= redact_value_at(child, rest);
        }
      }
      found
    }
    Value::Array(items) => items
      .iter_mut()
      .fold(false, |found, item| redact_value_at(item, path) | found),
    _ => false,
  }
}

/// The members of `members` that still hold the placeholder in `parts`, as paths: a recorded
/// redaction nothing replaced before the request was sent (ADR 0074 decision 6).
pub fn still_redacted(
  members: &[Member],
  parts: &BTreeMap<String, BTreeMap<String, SlotValue>>,
) -> Vec<String> {
  fn holds(value: &Value, path: &[String]) -> bool {
    match (path.split_first(), value) {
      (None, value) => contains_placeholder(value),
      (Some((name, rest)), Value::Object(members)) => members
        .iter()
        .any(|(member, child)| member.eq_ignore_ascii_case(name) && holds(child, rest)),
      (Some(_), Value::Array(items)) => items.iter().any(|item| holds(item, path)),
      _ => false,
    }
  }
  fn contains_placeholder(value: &Value) -> bool {
    match value {
      Value::String(text) => text == PLACEHOLDER,
      Value::Array(items) => items.iter().any(contains_placeholder),
      Value::Object(members) => members.values().any(contains_placeholder),
      _ => false,
    }
  }
  members
    .iter()
    .filter(|member| {
      parts.iter().any(|(part, slots)| {
        part.eq_ignore_ascii_case(&member.part)
          && slots.iter().any(|(slot, value)| {
            slot.eq_ignore_ascii_case(&member.slot)
              && match value.encoded.as_deref() {
                None | Some("json") => holds(&value.content, &member.path),
                Some(encoded) => {
                  value.content == placeholder_of(&Value::String(String::new()), Some(encoded))
                }
              }
          })
      })
    })
    .map(Member::to_path)
    .collect()
}

/// A captured value's absolute path (`$.request.headers['x-y'][0]`) as steps: a name for a member,
/// `None` for an element.
pub fn steps(path: &str) -> Vec<Option<String>> {
  let mut steps = Vec::new();
  let mut rest = path.strip_prefix('$').unwrap_or(path);
  while !rest.is_empty() {
    if let Some(quoted) = rest.strip_prefix("['") {
      let (name, remaining) = crate::plan::unquote(quoted);
      steps.push(Some(name));
      rest = remaining;
    } else if let Some(after) = rest.strip_prefix('[') {
      steps.push(None);
      rest = after.find(']').map_or("", |end| &after[end + 1..]);
    } else {
      let after = rest.strip_prefix('.').unwrap_or(rest);
      let end = after.find(['.', '[']).unwrap_or(after.len());
      steps.push(Some(after[..end].to_string()));
      rest = &after[end..];
    }
  }
  steps
}

// --- Reporting -------------------------------------------------------------------------------------

/// The secrets of one exchange (ADR 0074 decision 7): every string found under a redacted member, in
/// the interaction it ran and the values it carried. Each is replaced by [`PLACEHOLDER`] wherever it
/// appears in what is reported.
#[derive(Debug, Clone, Default)]
pub struct Secrets {
  found: Vec<String>,
}

impl Secrets {
  pub fn is_empty(&self) -> bool {
    self.found.is_empty()
  }

  fn add(&mut self, secret: &str) {
    if !secret.is_empty() && secret != PLACEHOLDER && !self.found.iter().any(|seen| seen == secret) {
      self.found.push(secret.to_string());
      // Longest first, so a secret that holds another is replaced whole.
      self
        .found
        .sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    }
  }

  /// Another exchange's secrets as well.
  pub fn extend(&mut self, other: &Secrets) {
    for secret in &other.found {
      self.add(secret);
    }
  }

  /// The strings under `members` in a document located at `at` (a part and slot, or less, or more).
  /// Steps that are elements are written as `None`: a member step passes through an array.
  pub fn collect(&mut self, members: &[Member], at: &[Option<&str>], value: &Value) {
    for member in members {
      let mut names = member.names();
      let mut inside = true;
      for step in at {
        match step {
          None => {}
          Some(step) => match names.next() {
            Some(name) if name.eq_ignore_ascii_case(step) => {}
            Some(_) => {
              inside = false;
              break;
            }
            // The document is under the member: all of it is the member's.
            None => break,
          },
        }
      }
      if inside {
        let rest: Vec<&str> = names.collect();
        self.collect_under(value, &rest);
      }
    }
  }

  fn collect_under(&mut self, value: &Value, path: &[&str]) {
    match (path.split_first(), value) {
      (None, value) => self.collect_all(value),
      (Some((name, rest)), Value::Object(members)) => {
        for (member, child) in members {
          if member.eq_ignore_ascii_case(name) {
            self.collect_under(child, rest);
          }
        }
      }
      (Some(_), Value::Array(items)) => items.iter().for_each(|item| self.collect_under(item, path)),
      _ => {}
    }
  }

  fn collect_all(&mut self, value: &Value) {
    match value {
      Value::String(text) => self.add(text),
      Value::Array(items) => items.iter().for_each(|item| self.collect_all(item)),
      Value::Object(members) => members.values().for_each(|member| self.collect_all(member)),
      _ => {}
    }
  }

  /// The strings under `members` in a set of parts, as a transport carries or a contract records them.
  pub fn collect_parts(&mut self, members: &[Member], parts: &BTreeMap<String, BTreeMap<String, SlotValue>>) {
    for (part, slots) in parts {
      for (slot, value) in slots {
        let at = [Some(part.as_str()), Some(slot.as_str())];
        match value.encoded.as_deref() {
          None | Some("json") => self.collect(members, &at, &value.content),
          // Octets have no members: the whole slot is a secret only when the whole slot is redacted.
          Some(encoded) => {
            let whole = members.iter().any(|member| {
              member.path.is_empty()
                && member.part.eq_ignore_ascii_case(part)
                && member.slot.eq_ignore_ascii_case(slot)
            });
            if whole && let Some(text) = value.content.as_str() {
              match encoded {
                "base64" => {
                  if let Some(decoded) = base64::engine::general_purpose::STANDARD
                    .decode(text)
                    .ok()
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                  {
                    self.add(&decoded);
                  }
                  self.add(text);
                }
                _ => self.add(text),
              }
            }
          }
        }
      }
    }
  }

  /// The example strings under `members` in an interaction's shapes: part -> slot -> shape.
  pub fn collect_shapes(&mut self, members: &[Member], parts: &BTreeMap<String, contract::ShapePart>) {
    for member in members {
      for (part, slots) in parts {
        if !part.eq_ignore_ascii_case(&member.part) {
          continue;
        }
        for (slot, shape) in slots {
          if slot.eq_ignore_ascii_case(&member.slot) {
            self.collect_shape_at(shape, &member.path);
          }
        }
      }
    }
  }

  fn collect_shape_at(&mut self, node: &Value, path: &[String]) {
    if !is_shape(node) {
      return;
    }
    let Some((name, rest)) = path.split_first() else {
      self.collect_shape(node);
      return;
    };
    if let Some(Value::Object(members)) = node.get("members") {
      for (member, child) in members {
        if member.eq_ignore_ascii_case(name) {
          self.collect_shape_at(child, rest);
        }
      }
    }
    for child in child_nodes_of(node) {
      self.collect_shape_at(child, path);
    }
  }

  fn collect_shape(&mut self, node: &Value) {
    if let Some(example) = node.get("example") {
      self.collect_all(example);
    }
    if let Some(Value::Object(named)) = node.get("members") {
      for child in named.values() {
        self.collect_shape(child);
      }
    }
    for child in child_nodes_of(node) {
      self.collect_shape(child);
    }
  }

  /// `text` with every secret replaced.
  pub fn scrub(&self, text: &str) -> String {
    let mut text = text.to_string();
    for secret in &self.found {
      if text.contains(secret.as_str()) {
        text = text.replace(secret.as_str(), PLACEHOLDER);
      }
    }
    text
  }

  /// Every string in a document scrubbed.
  pub fn scrub_json(&self, value: &mut Value) {
    if self.is_empty() {
      return;
    }
    match value {
      Value::String(text) => *text = self.scrub(text),
      Value::Array(items) => items.iter_mut().for_each(|item| self.scrub_json(item)),
      Value::Object(members) => members.values_mut().for_each(|member| self.scrub_json(member)),
      _ => {}
    }
  }

  fn scrub_runtime(&self, value: &mut RuntimeValue) {
    match value {
      RuntimeValue::String(text) => *text = self.scrub(text),
      RuntimeValue::Array(items) => items.iter_mut().for_each(|item| self.scrub_runtime(item)),
      RuntimeValue::Object(members) => members.values_mut().for_each(|member| self.scrub_runtime(member)),
      RuntimeValue::Entry { value, .. } => self.scrub_runtime(value),
      _ => {}
    }
  }

  fn scrub_marked(&self, marked: &mut Marked) {
    marked.map_texts(|text| self.scrub(text));
  }

  /// An executed plan with every secret replaced: its resolved values, its literals and its messages.
  pub fn scrub_executed(&self, executed: &mut Executed) {
    if self.is_empty() {
      return;
    }
    match &mut executed.result {
      Some(NodeResult::Value(value)) => self.scrub_runtime(value),
      Some(NodeResult::Error { message, marks, .. }) => {
        *message = self.scrub(message);
        if let Some(marks) = marks {
          self.scrub_marked(marks);
        }
      }
      _ => {}
    }
    match &mut executed.kind {
      ExecutedKind::Container { children, .. }
      | ExecutedKind::Action { children, .. }
      | ExecutedKind::Pipeline { children }
      | ExecutedKind::Splat { children } => children.iter_mut().for_each(|child| self.scrub_executed(child)),
      ExecutedKind::Value(literal) => self.scrub_json(&mut literal.value),
      ExecutedKind::Annotation { text } => *text = self.scrub(text),
      ExecutedKind::Resolve { .. } | ExecutedKind::ResolveCurrent { .. } => {}
    }
  }

  /// Mismatches with every secret replaced.
  pub fn scrub_mismatches(&self, mismatches: &mut [Mismatch]) {
    if self.is_empty() {
      return;
    }
    for mismatch in mismatches {
      mismatch.message = self.scrub(&mismatch.message);
      if let Some(marks) = &mut mismatch.marks {
        self.scrub_marked(marks);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  fn member(path: &str) -> Member {
    Member::parse(path).expect("parses")
  }

  #[test]
  fn a_path_is_a_part_a_slot_and_members() {
    assert_eq!(
      member("request.headers.authorization"),
      Member {
        part: "request".into(),
        slot: "headers".into(),
        path: vec!["authorization".into()]
      }
    );
    assert_eq!(member("request.body").path, Vec::<String>::new());
    assert_eq!(
      member("request.body['a.b'].c").path,
      vec!["a.b".to_string(), "c".to_string()]
    );
    assert_eq!(member("request.body['a.b'].c").to_path(), "request.body['a.b'].c");
  }

  #[test]
  fn a_path_that_is_not_one_is_refused() {
    for text in [
      "request",
      "",
      "request..headers",
      "request.body[0]",
      "request.body['open",
    ] {
      assert!(Member::parse(text).is_err(), "{text} should be refused");
    }
  }

  #[test]
  fn keep_takes_a_member_out_of_what_is_recorded_but_not_out_of_what_is_reported() {
    let session = SessionRedaction::parse(
      Some(&json!({ "add": ["request.body.password"], "keep": ["request.headers.Authorization"] })),
      "/config/redaction",
    )
    .expect("parses");
    let defaults = vec![
      member("request.headers.authorization"),
      member("response.headers.set-cookie"),
    ];
    let recorded: Vec<String> = session.recorded(&defaults).iter().map(Member::to_path).collect();
    assert_eq!(
      recorded,
      vec!["response.headers.set-cookie", "request.body.password"]
    );
    assert_eq!(session.reported(&defaults).len(), 3);
  }

  #[test]
  fn a_malformed_redaction_names_where() {
    let problems = SessionRedaction::parse(Some(&json!({ "add": ["request", 7] })), "/config/redaction")
      .expect_err("refused");
    let pointers: Vec<&str> = problems.iter().map(|problem| problem.pointer.as_str()).collect();
    assert_eq!(
      pointers,
      vec!["/config/redaction/add/0", "/config/redaction/add/1"]
    );
  }

  #[test]
  fn secrets_are_replaced_wherever_they_appear() {
    let mut secrets = Secrets::default();
    secrets.collect(
      &[member("request.headers.authorization")],
      &[Some("request"), Some("headers")],
      &json!({ "Authorization": ["Bearer s3cret"], "accept": ["*/*"] }),
    );
    assert_eq!(
      secrets.scrub("Expected 'Bearer x' but got 'Bearer s3cret'"),
      "Expected 'Bearer x' but got '[redacted]'"
    );
    assert_eq!(secrets.scrub("*/*"), "*/*");
  }

  #[test]
  fn a_document_under_the_member_is_all_secret() {
    let mut secrets = Secrets::default();
    secrets.collect(
      &[member("request.headers.authorization")],
      &[Some("request"), Some("headers"), Some("authorization"), None],
      &json!("Bearer s3cret"),
    );
    assert_eq!(secrets.scrub("Bearer s3cret"), PLACEHOLDER);
  }
}
