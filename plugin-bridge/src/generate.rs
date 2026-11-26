//! `<component>:generate-content` (ADR 0110 decision 7): a plugin's content generator, filling a whole
//! body before it is sent, as today's verifier passes a plugin request body with generators through the
//! plugin's `GenerateContent`.
//!
//! The value under test is the body as the pact records it. The action's one argument is an object: the
//! body's `content-type` as the pact records it, its `generators` in the v4 pact form, keyed by path as the
//! pact writes them, the plugin's `plugin-configuration` (`interaction`, the interaction's whole
//! configuration for the plugin, and `pact`), and the `test-context` the generators read, the exchange's
//! provider-state values. The request is built as pact-plugin-driver's `ContentGenerator::generate_content`
//! builds it for today's verifier.

use crate::compare::octets;
use crate::error::ComponentError;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_plugin_driver::proto::{self, body::ContentTypeHint};
use serde_json::{Map, Value, json};

/// What one `generate-content` application fills, read from its argument.
#[derive(Debug, Clone)]
pub struct Arguments {
  pub content_type: String,
  generators: Map<String, Value>,
  plugin_configuration: Option<(Value, Value)>,
  test_context: Map<String, Value>,
}

impl Arguments {
  /// Reads the argument from an application's `config` (spec §7.2): for this action, one object.
  pub fn read(config: Option<&Value>) -> Result<Arguments, ComponentError> {
    let argument = config
      .and_then(|config| config.get("arguments"))
      .and_then(Value::as_array)
      .and_then(|arguments| arguments.first())
      .and_then(Value::as_object)
      .ok_or_else(|| {
        ComponentError::invalid_config(
          "generate-content takes one argument, an object naming the body's content-type and its generators",
        )
      })?;
    let content_type = argument
      .get("content-type")
      .and_then(Value::as_str)
      .ok_or_else(|| ComponentError::invalid_config("generate-content's argument names no 'content-type'"))?
      .to_string();
    let object = |key: &str| match argument.get(key) {
      None | Some(Value::Null) => Ok(Map::new()),
      Some(Value::Object(members)) => Ok(members.clone()),
      Some(_) => Err(ComponentError::invalid_config(format!(
        "generate-content's '{key}' must be an object"
      ))),
    };
    let generators = object("generators")?;
    let test_context = object("test-context")?;
    let plugin_configuration = match argument.get("plugin-configuration") {
      None | Some(Value::Null) => None,
      Some(Value::Object(configuration)) => Some((
        configuration.get("interaction").cloned().unwrap_or(Value::Null),
        configuration.get("pact").cloned().unwrap_or(Value::Null),
      )),
      Some(_) => {
        return Err(ComponentError::invalid_config(
          "generate-content's 'plugin-configuration' must be an object with 'interaction' and 'pact'",
        ));
      }
    };
    Ok(Arguments {
      content_type,
      generators,
      plugin_configuration,
      test_context,
    })
  }

  /// The plugin's `GenerateContent` request for the body under test, built as the driver builds it: each
  /// generator its `type` and the rest of its members as its values, and no test mode, which the driver
  /// leaves unset.
  pub fn request(&self, value: &Value) -> Result<proto::GenerateContentRequest, ComponentError> {
    let body = octets(value, "the value under test")?;
    Ok(proto::GenerateContentRequest {
      contents: Some(proto::Body {
        content_type: self.content_type.clone(),
        content: Some(body),
        content_type_hint: ContentTypeHint::Default as i32,
      }),
      generators: self
        .generators
        .iter()
        .map(|(path, generator)| {
          let mut values = generator.as_object().cloned().unwrap_or_default();
          let kind = values
            .remove("type")
            .and_then(|kind| kind.as_str().map(str::to_string))
            .unwrap_or_default();
          (
            path.clone(),
            proto::Generator {
              r#type: kind,
              values: Some(structure(values)),
            },
          )
        })
        .collect(),
      plugin_configuration: self.plugin_configuration.as_ref().map(|(interaction, pact)| {
        proto::PluginConfiguration {
          interaction_configuration: interaction.as_object().cloned().map(structure),
          pact_configuration: pact.as_object().cloned().map(structure),
        }
      }),
      test_context: Some(structure(self.test_context.clone())),
      ..proto::GenerateContentRequest::default()
    })
  }
}

/// What the plugin generated, as a plan result document (plan-grammar spec §2.3): the body as a `value`
/// of octets. A plugin that answers with a failure, or with no body, is an `error`: the engine then sends
/// the body as recorded and says so, as today's verifier sends the original.
pub fn result(answer: Result<proto::GenerateContentResponse, anyhow::Error>) -> Value {
  match answer {
    Ok(proto::GenerateContentResponse {
      contents: Some(contents),
    }) => json!({
      "status": "value",
      "value": {
        "of": "bytes",
        "value": BASE64.encode(contents.content.unwrap_or_default()),
        "encoded": "base64"
      }
    }),
    Ok(_) => json!({ "status": "error", "message": "the plugin generated no body" }),
    Err(err) => {
      json!({ "status": "error", "message": format!("the plugin could not generate the body: {err:#}") })
    }
  }
}

fn structure(members: Map<String, Value>) -> prost_types::Struct {
  pact_plugin_driver::utils::to_proto_struct(&members.into_iter().collect())
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn the_request_is_what_the_driver_would_send() {
    let arguments = Arguments::read(Some(&json!({ "arguments": [{
      "content-type": "text/csv;charset=utf-8",
      "generators": { "column:3": { "type": "DateTime", "format": "yyyy-MM-dd" } },
      "plugin-configuration": { "interaction": { "request": { "csvHeaders": false } }, "pact": {} },
      "test-context": { "id": 7 }
    }] })))
    .unwrap();
    let request = arguments
      .request(&json!({ "content": "Name,100,2000-01-01\n", "encoded": "text" }))
      .unwrap();
    let contents = request.contents.unwrap();
    assert_eq!(contents.content_type, "text/csv;charset=utf-8");
    assert_eq!(contents.content.as_deref(), Some(&b"Name,100,2000-01-01\n"[..]));
    let generator = &request.generators["column:3"];
    assert_eq!(generator.r#type, "DateTime");
    assert!(generator.values.as_ref().unwrap().fields.contains_key("format"));
    assert!(!generator.values.as_ref().unwrap().fields.contains_key("type"));
    let configuration = request.plugin_configuration.unwrap();
    assert!(
      configuration
        .interaction_configuration
        .unwrap()
        .fields
        .contains_key("request")
    );
    assert!(request.test_context.unwrap().fields.contains_key("id"));
    assert_eq!(request.test_mode, 0, "the driver sets no test mode");
  }

  #[test]
  fn an_argument_that_cannot_be_read_is_invalid_config() {
    for argument in [
      json!("text/csv"),
      json!({}),
      json!({ "content-type": "text/csv", "generators": [] }),
    ] {
      let err =
        Arguments::read(Some(&json!({ "arguments": [argument.clone()] }))).expect_err(&argument.to_string());
      assert_eq!(err.code, "invalid-config", "{argument}");
    }
  }

  #[test]
  fn a_generated_body_is_a_value_of_octets_and_a_failure_an_error() {
    let generated = result(Ok(proto::GenerateContentResponse {
      contents: Some(proto::Body {
        content_type: "text/csv".into(),
        content: Some(b"a,1\n".to_vec()),
        content_type_hint: 0,
      }),
    }));
    assert_eq!(
      generated,
      json!({ "status": "value", "value": { "of": "bytes", "value": BASE64.encode("a,1\n"), "encoded": "base64" } })
    );
    assert_eq!(
      result(Ok(proto::GenerateContentResponse::default()))["status"],
      "error"
    );
    let failed = result(Err(anyhow::Error::from(tonic::Status::aborted("bad generator"))));
    assert_eq!(failed["status"], "error");
  }
}
