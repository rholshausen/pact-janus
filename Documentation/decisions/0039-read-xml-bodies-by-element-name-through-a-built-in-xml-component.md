# 0039 — Read XML bodies by element name, through a built-in XML component

- **Status**: accepted
- **Date**: 2026-10-22
- **Plan tasks**: S1.22 (the build); S1.25 (`janus upgrade`, which this leaves as it was); S1.23
  (multipart, whose parts reuse it)
- **Evidence**: pact-reference's matcher (`pact_matching::xml::match_xml` and its `compare_element`,
  `compare_attributes`, `compare_children` and `compare_text`; `MatchingContext::type_matcher_defined`);
  `pact_models`' `xml_utils::parse_bytes` and `generators::xml::XmlHandler`; pact-reference's V2 engine
  (`engine/bodies/xml.rs`, `xml_utils::resolve_path`); pact-jvm's `XmlContentMatcher` and
  `QualifiedName`; the 156 pact-specification cases the legacy harness skipped; S1.1's first run
  (`compatibility-suite/v1/19-…`); [ADR 0036](0036-read-text-bodies-through-a-text-component-and-unclaimed-bodies-as-bytes.md)
  decision 1; [ADR 0038](0038-read-form-bodies-as-a-query-string-through-a-built-in-form-component.md)'s
  tripwire

## Context

A v1–v4 pact with an XML body is refused as `component-unavailable`, and the legacy harness skips the
156 pact-specification cases that have one, the last cases it skipped. Today's verifier passes them.

Today's verifier parses both bodies with `sxd-document` and walks them from the root element. An element
is compared by the rule at its path, if any, or by its name. Then come its attributes, its child
elements and its text. Its paths are pact's own: `$.animals.alligator` for a child, with no index
however many there are, `['@name']` for an attribute, `['#text']` for text, and `<uri>:<local>` for a
name in a namespace. Children are grouped by name. A group that a type rule reaches is compared item
by item against the pact's first. Any other group is compared index by index. A request is closed: no
attribute or child the pact lacks. A response is open, except to a child that a type rule reaches. An
element's text is its text nodes, each trimmed, then joined, and it is compared even when the pact has
none. A rule on an element compares its name (`type`, `equality`, `regex`) or counts its child nodes
(`min`, `max`, `notEmpty`). Any other rule on an element fails.

One specification case says something else. `unexpected xml namespace` (v3 and v4) answers
`<alligator/>` with `<alligator xmlns="urn:alligators"/>` and says it matches. pact_matching's own
spec tests ignore that case as "Incorrect behaviour - fixed in V2 engine". pact-reference's V2 engine
finds an element by its local name when the pact gives it no namespace. pact-jvm compares the
qualified name, as today's Rust verifier does, and fails it.

The engine's document model has seven kinds (engine-protocol spec §2.4) and no element. ADR 0038's
tripwire asked the question again for XML: if a body kind needs a compiler case that is not an existing
slot's rules reused, does reading its v1–v4 paths belong with its component?

## Decision

1. **XML is a built-in content component, `xml`**, in `engine/component-xml`. It declares the types
   today's verifier counts as XML: an `application` or `text` type whose subtype is `xml` or ends in
   `+xml`. Every embedding registers it after JSON and before text, so `text/xml` reaches it and not
   the `text/*` component (ADR 0036 decision 1). `engine/hello` lists `content/xml`. It parses with
   `sxd-document`, as today's verifier does, so both accept the same documents. It contributes no plan
   fragments and detects nothing.
2. **An XML body's document is keyed by element name.** The body is an object with one member, the
   root element's local name, holding the root element. An element is an object of:
   - `#namespace`, its namespace URI, when it is in one;
   - `@<name>`, each attribute's value, in name order. A namespaced attribute is written in Clark
     notation, `@{<uri>}<local>`. Namespace declarations are not attributes;
   - `#text`, its text nodes, each trimmed, then joined, as today's verifier reads them. It is absent
     when that is empty;
   - `<local name>`, an array of the child elements with that local name, in document order. There is
     one member per name, in the order the names first appear.

   `encode` writes the inverse: a declaration, a default-namespace declaration wherever the namespace
   changes, a generated prefix for each namespaced attribute, and an element's text before its
   children. Numbers and booleans are written as their text. Comments, processing instructions,
   prefixes and the order between different names' children are not kept. Today's verifier reads none
   of them either, so nothing a verdict reads is lost.
3. **The legacy layer reads a v1–v4 XML body into that document, and the legacy compiler walks it as
   today's verifier walks XML.** The pact's side is parsed with `pact_models::xml_utils::parse_bytes`.
   A reply is decoded by the component, as the pact's type (ADR 0036 decision 4). A test holds the two
   readers to the same document for every XML body in the repository. Rules are looked up by pact's own
   paths, which the compiler keeps beside the document's. Where today's verifier asks a parsed element,
   the plan asks the document. Every pact-specification case passes, and none is skipped.
4. **An element the pact puts in a namespace must be in that namespace. One the pact puts in no
   namespace matches its local name in any namespace.** This is the specification's case and the V2
   engine's behaviour. Since children are keyed by local name, it is an absence of a check, not a
   search. Against today's Rust verifier and pact-jvm it is a deliberate difference, which S1.1's
   ledger records. Siblings that share a local name in different namespaces are one group.
5. **A type rule on text reads the text, as today's verifier does.** `number` and `decimal` mean the text
   parses as a number, `integer` as an unsigned integer, and `boolean` that it is `true` or `false`. This
   is the new `legacy:parses-as`, and it applies to every v1–v4 value that is text: an attribute,
   element text, a path, a header, a query or form value. Before this, `match:number` compared the
   value's kind, so an `integer` rule on a query parameter failed where today's verifier passes it.
6. **An XML body's request generators are applied** with `pact_models`' `XmlHandler`, as today's
   verifier applies them, asking the `pact` generator component (ADR 0034). The document is written
   back with `sxd-document`'s writer, as today's verifier writes it.
7. **Plan grammar v1 gains three legacy actions and a quoted path segment.** `legacy:xml-children`
   gives an element's child names, one per child element, for a size rule and `notEmpty`.
   `legacy:xml-only-children` fails a child the pact does not name, with today's message, `Unexpected
   child <cat/>`. `legacy:parses-as` is decision 5. A member whose name a plain path segment cannot
   hold is written `['…']`, as in `['@{urn:names}name']`, and `navigate` reads it.
8. **ADR 0038's tripwire fired, and the v1–v4 paths stay with the legacy compiler.** XML needs a walk
   of its own, not a reused slot's rules. Reading XML belongs to the component: it parses the octets
   and owns the document, which component-interfaces spec §6.1 now specifies. Interpreting v1–v4
   paths and rules over that document stays where plan-grammar spec §1 puts it. A component that
   compiled them would interpret `pact_models` rules, and it would hold a second copy of v1–v4
   precedence. The kernel learns the document's three member conventions (`@`, `#text`, `#namespace`)
   in the legacy compiler and two legacy actions, and nowhere else.

## Alternatives considered

- **A node model: an element as `{name, namespace, attributes, children: [...]}`, with text and
  elements interleaved.** It keeps sibling order and mixed content. But pact's XML paths would no longer
  be document paths, so finding a child by name would need an action at every step. A contract's XML
  shape would also be a list of differently named elements, which `each-like` cannot describe. Today's
  verifier groups children by name and joins text, so the extra fidelity reaches no verdict.
- **Children keyed by qualified name.** Namespaces would be in the keys, and two namespaces' children of
  one local name would stay apart. But an element the pact puts in no namespace would then need a
  search through the keys wherever it is found (decision 4). That means a lookup action in place of a
  path at every step of almost every XML plan, since most XML pacts name no namespace.
- **Compare qualified names, as today's verifiers do.** Every verdict would be today's. But a
  specification case would have to be skipped, and pact-reference itself calls today's behaviour there
  incorrect. S1.22's exit is no case skipped.
- **The XML component compiles v1–v4 rules into a plan fragment.** This is the tripwire's literal
  answer, rejected for the reasons ADR 0038 gave for form: a component interpreting `pact_models` rules,
  and v1–v4 precedence in two places.
- **`quick-xml` or `roxmltree`.** Both are better maintained than `sxd-document`. But they accept and
  reject different documents from today's verifier, and `sxd-document` is already in the tree through
  `pact_models`.

## Consequences

- A v1–v4 pact with an XML body verifies as it does today, and its request generators are applied. The
  legacy harness runs all 741 specification cases, with no exclusion rule and no `janus:skip`.
- A Janus contract can declare an XML body and shape it in the model of decision 2. A mock serves it
  with children grouped by name and text before children.
- Deliberate differences from today's verifier, besides decision 4:
  - a size rule on an element counts its child elements. Today's verifier counts child nodes, the
    whitespace between elements included, so a pretty-printed body passes a `min` it fails compact;
  - a root element with another name is a mismatch, even under a `regex` rule on the root's path,
    because the plan finds the root by its name.
- Mismatch messages are the engine's, not today's, except `Unexpected child`.
- An `integer`, `number`, `decimal` or `boolean` rule on a path, header or query value now passes text
  that parses, as today. No corpus plan changed.
- `janus upgrade` still carries an XML body across with `body-not-parsed`. S1.25 converts XML bodies
  too, and its shapes are written in decision 2's model.
- A Janus contract's XML text is read as a pact's is: each text node trimmed, a CDATA section's
  contents included, since `sxd-document` reads CDATA as text. Whitespace XML counts is lost, and a
  mock cannot serve it. Keeping text as written, in contracts only, is S2.10, which amends decision 2.
- Plan text: an XML body's plan shows each element as a container, attributes as `.@name`, text as
  `.#text`, a type-matched group as a `flow:for-each`, and namespaced names in quotes.
- **Tripwire**: if a contract needs mixed content, or siblings of different names served in their
  order, the document gains a member that records the order. That is an additive change to decision 2,
  not a new model.
