//! Merging contracts, and the order every contract's interactions are written in (contract-file spec
//! §2.4, §4.2; ADR 0065). Several test files record one consumer and provider into one file: an
//! interaction is its description with its states, two recorded the same are one, and two recorded
//! differently are a conflict. Interactions are sorted by that identity, so a merged file's bytes do
//! not depend on which writer came first. The components each writer loaded are joined too (ADR 0097).

use super::model::{Contract, Interaction, Metadata, RecordedComponent};
use std::cmp::Ordering;

/// Why two contracts cannot be merged.
#[derive(Debug, Clone, PartialEq)]
pub enum Conflict {
  /// The file holds another consumer's or provider's contract.
  Parties { consumer: String, provider: String },
  /// Interactions with one identity, recorded differently: each one's description and states.
  Interactions(Vec<Interaction>),
}

/// Sorts interactions into identity order: by description, in code-point order, then by the
/// canonical bytes of their states, a missing `states` first.
pub fn sort(interactions: &mut [Interaction]) {
  interactions.sort_by(compare);
}

fn compare(a: &Interaction, b: &Interaction) -> Ordering {
  a.description
    .cmp(&b.description)
    .then_with(|| states_bytes(a).cmp(&states_bytes(b)))
}

/// The canonical bytes of an interaction's states, `None` when it has none.
fn states_bytes(interaction: &Interaction) -> Option<Vec<u8>> {
  interaction
    .states
    .as_ref()
    .map(|states| serde_json::to_vec(states).unwrap_or_default())
}

fn same_identity(a: &Interaction, b: &Interaction) -> bool {
  compare(a, b) == Ordering::Equal
}

fn record(interaction: &Interaction) -> Vec<u8> {
  serde_json::to_vec(interaction).unwrap_or_default()
}

/// `existing`'s interactions and `new`'s, joined by identity and in identity order. The header is
/// `new`'s. An interaction both hold must be the same bytes in both.
pub fn merge(existing: Contract, new: Contract) -> Result<Contract, Conflict> {
  if existing.consumer.name != new.consumer.name || existing.provider.name != new.provider.name {
    return Err(Conflict::Parties {
      consumer: existing.consumer.name,
      provider: existing.provider.name,
    });
  }
  let mut interactions = new.interactions.clone();
  let mut conflicts = Vec::new();
  for kept in existing.interactions {
    match new.interactions.iter().find(|i| same_identity(i, &kept)) {
      None => interactions.push(kept),
      Some(recorded) if record(recorded) == record(&kept) => {}
      Some(_) => conflicts.push(kept),
    }
  }
  if !conflicts.is_empty() {
    sort(&mut conflicts);
    return Err(Conflict::Interactions(conflicts));
  }
  sort(&mut interactions);
  let metadata = merge_metadata(existing.metadata, new.metadata.clone());
  Ok(Contract {
    interactions,
    metadata,
    ..new
  })
}

/// `new`'s metadata, listing every component either writer loaded (ADR 0097 decision 6): the file
/// holds interactions recorded by each, so two builds of one component both stay.
fn merge_metadata(existing: Option<Metadata>, new: Option<Metadata>) -> Option<Metadata> {
  let kept = existing.and_then(|m| m.components).unwrap_or_default();
  if kept.is_empty() {
    return new;
  }
  let mut metadata = new.unwrap_or(Metadata {
    writer: None,
    created: None,
    build_url: None,
    components: None,
  });
  let mut components = metadata.components.take().unwrap_or_default();
  components.extend(kept);
  RecordedComponent::normalise(&mut components);
  metadata.components = Some(components);
  Some(metadata)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contract::{Party, RecordedSelection, RecordedSource, State, write_canonical};
  use pretty_assertions::assert_eq;
  use serde_json::json;
  use std::collections::BTreeMap;

  fn interaction(description: &str, states: Option<&[&str]>, status: i64) -> Interaction {
    Interaction {
      description: description.to_string(),
      transport: None,
      states: states.map(|names| {
        names
          .iter()
          .map(|name| serde_json::from_value::<State>(json!({ "name": name })).unwrap())
          .collect()
      }),
      parts: BTreeMap::from([(
        "response".to_string(),
        BTreeMap::from([(
          "status".to_string(),
          json!({ "shape": "equality", "example": status }),
        )]),
      )]),
      content_types: None,
      requires: None,
      redacted: None,
      policy: None,
      selection: RecordedSelection {
        variants: vec![],
        report: BTreeMap::new(),
      },
    }
  }

  fn contract(consumer: &str, interactions: Vec<Interaction>) -> Contract {
    Contract {
      format: crate::contract::FORMAT.to_string(),
      schema: None,
      consumer: Party {
        name: consumer.to_string(),
      },
      provider: Party {
        name: "orders-api".to_string(),
      },
      interactions,
      metadata: None,
    }
  }

  fn descriptions(contract: &Contract) -> Vec<(String, Option<usize>)> {
    contract
      .interactions
      .iter()
      .map(|i| (i.description.clone(), i.states.as_ref().map(Vec::len)))
      .collect()
  }

  #[test]
  fn identity_order_is_by_description_then_states_with_none_first() {
    let mut interactions = vec![
      interaction("b", None, 200),
      interaction("a", Some(&["x", "y"]), 200),
      interaction("a", Some(&["x"]), 200),
      interaction("a", None, 200),
      interaction("Z", None, 200),
    ];
    sort(&mut interactions);
    let order: Vec<_> = interactions
      .iter()
      .map(|i| (i.description.as_str(), i.states.as_ref().map(Vec::len)))
      .collect();
    // Code-point order puts upper case first. States compare as bytes: `[{"name":"x"},…]` sorts
    // before `[{"name":"x"}]`, since `,` is below `]`.
    assert_eq!(
      order,
      vec![
        ("Z", None),
        ("a", None),
        ("a", Some(2)),
        ("a", Some(1)),
        ("b", None)
      ]
    );
  }

  #[test]
  fn merging_joins_by_identity_whichever_contract_came_first() {
    let a = contract(
      "web",
      vec![interaction("get", None, 200), interaction("delete", None, 204)],
    );
    let b = contract(
      "web",
      vec![interaction("put", None, 201), interaction("get", None, 200)],
    );
    let one = merge(a.clone(), b.clone()).unwrap();
    let other = merge(b, a).unwrap();
    assert_eq!(
      descriptions(&one),
      vec![
        ("delete".into(), None),
        ("get".into(), None),
        ("put".into(), None)
      ]
    );
    assert_eq!(write_canonical(&one).unwrap(), write_canonical(&other).unwrap());
  }

  #[test]
  fn the_same_description_with_other_states_is_another_interaction() {
    let a = contract("web", vec![interaction("get", Some(&["exists"]), 200)]);
    let b = contract("web", vec![interaction("get", Some(&["missing"]), 404)]);
    assert_eq!(merge(a, b).unwrap().interactions.len(), 2);
  }

  #[test]
  fn an_interaction_recorded_differently_is_a_conflict_naming_it() {
    let a = contract(
      "web",
      vec![interaction("get", None, 200), interaction("put", None, 201)],
    );
    let b = contract(
      "web",
      vec![interaction("get", None, 404), interaction("put", None, 201)],
    );
    let Err(Conflict::Interactions(conflicts)) = merge(a, b) else {
      panic!("a conflict");
    };
    assert_eq!(
      conflicts
        .iter()
        .map(|i| i.description.as_str())
        .collect::<Vec<_>>(),
      vec!["get"]
    );
  }

  fn with_components(mut contract: Contract, components: &[(&str, &str)]) -> Contract {
    contract.metadata = Some(Metadata {
      writer: None,
      created: None,
      build_url: None,
      components: Some(
        components
          .iter()
          .map(|(name, digest)| RecordedComponent {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            source: RecordedSource {
              kind: "file".to_string(),
              reference: None,
              digest: Some(digest.to_string()),
              signer: None,
            },
          })
          .collect(),
      ),
    });
    contract
  }

  fn listed(contract: &Contract) -> Vec<(String, String)> {
    contract
      .metadata
      .iter()
      .flat_map(|m| m.components.iter().flatten())
      .map(|c| (c.name.clone(), c.source.digest.clone().unwrap_or_default()))
      .collect()
  }

  #[test]
  fn merging_lists_every_component_either_writer_loaded_once() {
    let a = with_components(
      contract("web", vec![interaction("get", None, 200)]),
      &[("csv", "sha256:bb"), ("xml2", "sha256:cc")],
    );
    let b = with_components(
      contract("web", vec![interaction("put", None, 201)]),
      &[("csv", "sha256:aa"), ("xml2", "sha256:cc")],
    );
    let one = merge(a.clone(), b.clone()).unwrap();
    // Two builds of `csv` both took part, so both stay; `xml2`'s one build is listed once.
    assert_eq!(
      listed(&one),
      vec![
        ("csv".into(), "sha256:aa".into()),
        ("csv".into(), "sha256:bb".into()),
        ("xml2".into(), "sha256:cc".into()),
      ]
    );
    assert_eq!(
      write_canonical(&one).unwrap(),
      write_canonical(&merge(b, a).unwrap()).unwrap()
    );
  }

  #[test]
  fn merging_into_a_contract_with_no_components_keeps_the_ones_it_had() {
    let a = with_components(
      contract("web", vec![interaction("get", None, 200)]),
      &[("csv", "sha256:aa")],
    );
    let b = contract("web", vec![interaction("put", None, 201)]);
    assert_eq!(
      listed(&merge(a, b.clone()).unwrap()),
      vec![("csv".into(), "sha256:aa".into())]
    );
    assert_eq!(merge(b.clone(), b).unwrap().metadata, None);
  }

  #[test]
  fn another_pairs_contract_is_a_conflict() {
    let a = contract("mobile", vec![interaction("get", None, 200)]);
    let b = contract("web", vec![interaction("get", None, 200)]);
    assert_eq!(
      merge(a, b),
      Err(Conflict::Parties {
        consumer: "mobile".into(),
        provider: "orders-api".into()
      })
    );
  }
}
