//! The plan document model (plan-grammar spec §2, `schemas/v0/plan.schema.json`).
//!
//! A [`Node`]'s `result` (spec §2.3) is not modelled here: nothing in plan task 3.3 executes a
//! plan or attaches one, and adding the field speculatively would just be a place for `None` to
//! live until the interpreter (task 3.4) has an opinion about it.

use serde_json::Value;

/// The grammar version every [`Plan`] this compiler produces is written against (spec §7.1).
pub const GRAMMAR_VERSION: &str = "v0";

/// A compiled matching plan (spec §2): a rendering, not a record (spec §7) — nothing outside this
/// process stores one.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
  pub grammar: &'static str,
  pub root: Node,
  /// The variant id this plan was compiled under (design 2.3), when it was compiled under one.
  /// `None` means the shape was compiled unpinned.
  pub variant: Option<String>,
}

/// One plan node (spec §2.1). `kind` decides which carrier applies, mirroring
/// `plan.schema.json`'s `Node` where exactly one of the kind-specific members is present.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
  pub kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
  /// Executes by executing its children; its result is their conjunction (spec §2.1). `label` is
  /// what makes a plan inspectable — deleting it would leave a plan that explains nothing — but it
  /// is optional: a container that exists purely to bundle several nodes into one carries none.
  Container {
    label: Option<String>,
    children: Vec<Node>,
  },
  /// Applies the named action to its children (spec §4). Unnamespaced names are reserved for the
  /// core action set; `<component>:<name>` is contributed (spec §4.1).
  Action { name: String, children: Vec<Node> },
  /// Yields the literal it carries (spec §2.2).
  Value(Literal),
  /// Resolves `path` against the interaction context (spec §2.1).
  Resolve { path: String },
  /// Resolves `path` against the current item on the iteration stack (spec §2.1) — the companion
  /// of `for-each` and `pipeline`, the only two constructs that make anything current.
  ResolveCurrent { path: String },
  /// Applies each child to the next, yielding the last (spec §2.1).
  Pipeline { children: Vec<Node> },
  /// Executes its children and replaces itself with their results (spec §2.1) — the variadic
  /// escape for an action whose argument count depends on the value being matched.
  Splat { children: Vec<Node> },
  /// Prose. Not executable (spec §2.1).
  Annotation { text: String },
}

impl Node {
  pub fn container(label: Option<String>, children: Vec<Node>) -> Node {
    Node {
      kind: NodeKind::Container { label, children },
    }
  }

  pub fn action(name: impl Into<String>, children: Vec<Node>) -> Node {
    Node {
      kind: NodeKind::Action {
        name: name.into(),
        children,
      },
    }
  }

  pub fn value(literal: Literal) -> Node {
    Node {
      kind: NodeKind::Value(literal),
    }
  }

  pub fn resolve(path: impl Into<String>) -> Node {
    Node {
      kind: NodeKind::Resolve { path: path.into() },
    }
  }

  pub fn resolve_current(path: impl Into<String>) -> Node {
    Node {
      kind: NodeKind::ResolveCurrent { path: path.into() },
    }
  }

  pub fn pipeline(children: Vec<Node>) -> Node {
    Node {
      kind: NodeKind::Pipeline { children },
    }
  }

  pub fn splat(children: Vec<Node>) -> Node {
    Node {
      kind: NodeKind::Splat { children },
    }
  }

  pub fn annotation(text: impl Into<String>) -> Node {
    Node {
      kind: NodeKind::Annotation { text: text.into() },
    }
  }
}

/// The document-model kind a `value` node's literal carries, plus the one plan-only kind: `entry`
/// (spec §2.2), a key paired with a value, which iteration over an object's members needs and
/// which the document model itself has no term for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
  Null,
  Boolean,
  Number,
  String,
  Array,
  Object,
  Bytes,
  Entry,
}

/// A literal carried by a `value` node (spec §2.2, `plan.schema.json`'s `Value`).
#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
  pub of: DocumentKind,
  pub value: Value,
  /// Representation tag (protocol spec §2.4-2.5): absent means `value` is its natural JSON form,
  /// `"base64"` means `value` is the base64 of an octet sequence.
  pub encoded: Option<String>,
  /// For `of: Entry` only: the key its value is paired with.
  pub key: Option<String>,
}

impl Literal {
  pub fn null() -> Literal {
    Literal {
      of: DocumentKind::Null,
      value: Value::Null,
      encoded: None,
      key: None,
    }
  }

  pub fn string(s: impl Into<String>) -> Literal {
    Literal {
      of: DocumentKind::String,
      value: Value::String(s.into()),
      encoded: None,
      key: None,
    }
  }

  pub fn number(n: u64) -> Literal {
    Literal {
      of: DocumentKind::Number,
      value: Value::from(n),
      encoded: None,
      key: None,
    }
  }

  /// From a plain JSON literal (an `any-of` option, a `one-of` discriminator value, ...): `of` is
  /// inferred from the value's own kind. Never produces `Bytes` — only a shape's tagged `example`
  /// (spec §2.2) carries that distinction; see [`Literal::from_example`].
  pub fn from_json(value: &Value) -> Literal {
    let of = match value {
      Value::Null => DocumentKind::Null,
      Value::Bool(_) => DocumentKind::Boolean,
      Value::Number(_) => DocumentKind::Number,
      Value::String(_) => DocumentKind::String,
      Value::Array(_) => DocumentKind::Array,
      Value::Object(_) => DocumentKind::Object,
    };
    Literal {
      of,
      value: value.clone(),
      encoded: None,
      key: None,
    }
  }

  /// From a shape's `example` (spec §3.3), honouring its bytes tag (protocol spec §2.5).
  pub fn from_example(example: &crate::shape::Example) -> Literal {
    match example.encoded.as_deref() {
      Some(tag @ "base64") => Literal {
        of: DocumentKind::Bytes,
        value: example.value.clone(),
        encoded: Some(tag.to_string()),
        key: None,
      },
      _ => Literal::from_json(&example.value),
    }
  }
}
