//! Merging contracts, and the order every contract's interactions are written in (contract-file spec
//! §2.4, §4.2; ADR 0065). Several test files record one consumer and provider into one file: an
//! interaction is its description with its states, two recorded the same are one, and two recorded
//! differently are a conflict. Interactions are sorted by that identity, so a merged file's bytes do
//! not depend on which writer came first.

use super::model::{Contract, Interaction};
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
  Ok(Contract { interactions, ..new })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contract::{Party, RecordedSelection, State, write_canonical};
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
