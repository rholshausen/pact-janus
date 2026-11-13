//! AsyncAPI 2.x and 3.0 to a provider shape (ADR 0091, on ADR 0090's identity).
//!
//! Only what the documented application produces is its shape: a message it sends, and the reply it
//! answers a request it receives with. Each is an entry whose `message` part is the message's
//! `metadata` (its headers, and the content type Pact's message model reads there) and `body` (its
//! payload), through the same schema mapping as OpenAPI's. An entry is selected by the values the
//! message fixes, a header or payload member whose schema allows one value, since that is what a
//! consumer's recorded message can carry (ADR 0090 decision 7); a reply's by its request's. The
//! channel's address is a selector member only where a header fixes it.

use super::schema::{Deriver, escape};
use super::{Loss, Options, is_json};
use serde_json::{Map, Value, json};

/// How many references are followed to reach one object before it is unresolved: more than any
/// document chains, few enough that a cycle of references ends.
const MAX_REFERENCES: usize = 16;

/// How deep a schema is searched for the values it fixes.
const MAX_FIXED_DEPTH: usize = 32;

/// The AsyncAPI major versions read: 2.x names what the application sends `subscribe`, and 3.0
/// names it `send`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Version {
  Two,
  Three,
}

/// The interactions of a provider shape, from an AsyncAPI document whose version has been checked.
pub fn interactions(
  document: &Value,
  version: Version,
  options: &Options,
  losses: &mut Vec<Loss>,
) -> Vec<Value> {
  let mut deriver = Deriver::new(document, options, losses);
  let operations = match version {
    Version::Two => deriver.operations_2(),
    Version::Three => deriver.operations_3(),
  };
  let mut entries = Vec::new();
  for operation in &operations {
    entries.extend(deriver.entries(operation, version));
  }
  deriver.ties(&entries);
  entries.into_iter().map(|entry| entry.json).collect()
}

/// A message as an operation names it: its references followed and its traits applied.
struct Message {
  name: String,
  pointer: String,
  object: Value,
}

/// What the application produces on one address: messages it sends, or replies to the requests it
/// receives there.
struct Operation {
  id: Option<String>,
  address: String,
  /// What it sends, or, with `replies`, the requests it answers.
  messages: Vec<Message>,
  replies: Option<Vec<Message>>,
}

struct Entry {
  description: String,
  pointer: String,
  /// The selector's fixed values by where they are, for telling which entries tie.
  fixed: Vec<(String, Value)>,
  json: Value,
}

/// A message's payload, as far as a body can be derived from it.
enum Payload {
  Schema(Value, String),
  Absent,
  /// A schema in a format the mapping does not read (Avro, protobuf, RAML).
  Format(String, String),
  /// A content type that is not JSON.
  NotJson(String),
}

impl Deriver<'_> {
  /// AsyncAPI 2.x: each channel's `subscribe` operation is what the application publishes there.
  /// Its `publish` is what it receives, the other side's shape.
  fn operations_2(&mut self) -> Vec<Operation> {
    let mut operations = Vec::new();
    let Some(channels) = self.document.get("channels").and_then(Value::as_object) else {
      return operations;
    };
    for (address, channel) in channels {
      let pointer = format!("/channels/{}", escape(address));
      let Some((channel, pointer, _)) = self.follow(channel, &pointer) else {
        continue;
      };
      let Some(operation) = channel.get("subscribe") else {
        continue;
      };
      let pointer = format!("{pointer}/subscribe");
      let id = operation
        .get("operationId")
        .and_then(Value::as_str)
        .map(str::to_string);
      let messages = match operation.get("message") {
        Some(message) => match message.get("oneOf").and_then(Value::as_array) {
          Some(alternatives) => alternatives
            .iter()
            .enumerate()
            .filter_map(|(index, message)| {
              self.message(
                message,
                &format!("{pointer}/message/oneOf/{index}"),
                &format!("message-{index}"),
                Version::Two,
              )
            })
            .collect(),
          None => self
            .message(message, &format!("{pointer}/message"), "message", Version::Two)
            .into_iter()
            .collect(),
        },
        None => Vec::new(),
      };
      operations.push(Operation {
        id,
        address: address.clone(),
        messages,
        replies: None,
      });
    }
    operations
  }

  /// AsyncAPI 3.0: an operation whose action is `send` is a message the application sends, and one
  /// whose action is `receive` with a `reply` is a request it answers. A `receive` with no reply is
  /// what it consumes, and a `send` with one a request it makes: the other side's shapes.
  fn operations_3(&mut self) -> Vec<Operation> {
    let mut operations = Vec::new();
    let Some(declared) = self.document.get("operations").and_then(Value::as_object) else {
      return operations;
    };
    for (id, operation) in declared {
      let pointer = format!("/operations/{}", escape(id));
      let Some((operation, pointer, _)) = self.follow(operation, &pointer) else {
        continue;
      };
      let action = operation.get("action").and_then(Value::as_str);
      let reply = operation.get("reply");
      let answers = match (action, reply) {
        (Some("send"), None) => false,
        (Some("receive"), Some(_)) => true,
        _ => continue,
      };
      let Some((channel, channel_pointer)) = self.channel_3(operation.get("channel"), &pointer) else {
        continue;
      };
      let address = address_3(&channel, &channel_pointer);
      let messages = self.messages_3(&operation, &pointer, &channel, &channel_pointer);
      let replies = match reply {
        Some(reply) if answers => match self.replies_3(reply, &format!("{pointer}/reply")) {
          Some(replies) => Some(replies),
          None => continue,
        },
        _ => None,
      };
      operations.push(Operation {
        id: Some(id.clone()),
        address,
        messages,
        replies,
      });
    }
    operations
  }

  fn channel_3(&mut self, channel: Option<&Value>, operation_pointer: &str) -> Option<(Value, String)> {
    let Some(channel) = channel else {
      self.lose(
        "reference-unresolved",
        operation_pointer,
        "the operation names no channel, so nothing it sends is derived",
      );
      return None;
    };
    self
      .follow(channel, &format!("{operation_pointer}/channel"))
      .map(|(channel, pointer, _)| (channel, pointer))
  }

  /// An operation's messages: those it lists, else every message of its channel.
  fn messages_3(
    &mut self,
    operation: &Value,
    pointer: &str,
    channel: &Value,
    channel_pointer: &str,
  ) -> Vec<Message> {
    if let Some(listed) = operation.get("messages").and_then(Value::as_array) {
      return listed
        .iter()
        .enumerate()
        .filter_map(|(index, message)| {
          self.message(
            message,
            &format!("{pointer}/messages/{index}"),
            &format!("message-{index}"),
            Version::Three,
          )
        })
        .collect();
    }
    let Some(messages) = channel.get("messages").and_then(Value::as_object) else {
      return Vec::new();
    };
    messages
      .iter()
      .filter_map(|(key, message)| {
        self.message(
          message,
          &format!("{channel_pointer}/messages/{}", escape(key)),
          key,
          Version::Three,
        )
      })
      .collect()
  }

  /// A reply's messages: those it lists, else every message of its channel. A reply whose address
  /// is only known at run time names its messages, or none can be derived.
  fn replies_3(&mut self, reply: &Value, pointer: &str) -> Option<Vec<Message>> {
    let (reply, pointer, _) = self.follow(reply, pointer)?;
    let channel = match reply.get("channel") {
      Some(channel) => self.channel_3(Some(channel), &pointer),
      None => None,
    };
    let (channel, channel_pointer) = channel.unwrap_or((Value::Null, pointer.clone()));
    let replies = self.messages_3(&reply, &pointer, &channel, &channel_pointer);
    if replies.is_empty() {
      self.lose(
        "reply-unresolved",
        &pointer,
        "the reply names no message, and no channel whose messages it is, so the operation's \
         answers are not derived",
      );
      return None;
    }
    Some(replies)
  }

  /// A message, its references followed and its traits applied, named by its `name`, else its
  /// `messageId`, else the last reference followed to it, else its key.
  fn message(&mut self, value: &Value, pointer: &str, key: &str, version: Version) -> Option<Message> {
    let (mut object, pointer, reference) = self.follow(value, pointer)?;
    if let Some(traits) = object.get("traits").and_then(Value::as_array).cloned() {
      object.as_object_mut().map(|object| object.remove("traits"));
      let mut traited = match version {
        // 2.x: each trait is merged into the message in turn, so a trait wins.
        Version::Two => object.clone(),
        // 3.0: the message's own members win over its traits'.
        Version::Three => json!({}),
      };
      for (index, item) in traits.iter().enumerate() {
        if let Some((item, _, _)) = self.follow(item, &format!("{pointer}/traits/{index}")) {
          merge_patch(&mut traited, &item);
        }
      }
      if version == Version::Three {
        merge_patch(&mut traited, &object);
      }
      object = traited;
    }
    let name = ["name", "messageId"]
      .iter()
      .find_map(|member| object.get(*member).and_then(Value::as_str).map(str::to_string))
      .or_else(|| reference.and_then(|reference| reference.rsplit('/').next().map(unescape)))
      .unwrap_or_else(|| key.to_string());
    Some(Message {
      name,
      pointer,
      object,
    })
  }

  /// A value that may be a `$ref`, followed within the document until it is not one, with the
  /// pointer to what it is and the last reference followed. One that cannot be followed is reported.
  fn follow(&mut self, value: &Value, pointer: &str) -> Option<(Value, String, Option<String>)> {
    let mut value = value.clone();
    let mut pointer = pointer.to_string();
    let mut last = None;
    for _ in 0..MAX_REFERENCES {
      let Some(reference) = value.get("$ref").and_then(Value::as_str).map(str::to_string) else {
        return Some((value, pointer, last));
      };
      let Some(target) = self.lookup(&reference).cloned() else {
        let code = if reference.starts_with('#') {
          "reference-unresolved"
        } else {
          "external-reference"
        };
        self.lose(
          code,
          &pointer,
          format!("'{reference}' is not in this document, so what it names is not derived"),
        );
        return None;
      };
      value = target;
      pointer = reference[1..].to_string();
      last = Some(reference);
    }
    self.lose(
      "reference-unresolved",
      &pointer,
      format!("more than {MAX_REFERENCES} references are followed to reach it, so it is not derived"),
    );
    None
  }

  /// The entries one operation contributes: one per message it sends, or per request and reply.
  fn entries(&mut self, operation: &Operation, version: Version) -> Vec<Entry> {
    let mut entries = Vec::new();
    match &operation.replies {
      None => {
        for message in &operation.messages {
          let description = format!("send {} {}", operation.address, message.name);
          self.entry = description.clone();
          let (slots, fixed) = self.message_slots(message, version);
          entries.push(self.entry_of(
            description,
            message,
            ("message", fixed),
            json!({ "message": slots }),
            self.source(operation, "send", &message.name, None),
          ));
        }
      }
      Some(replies) => {
        for request in &operation.messages {
          for reply in replies {
            let description = format!(
              "reply to {} on {} with {}",
              request.name, operation.address, reply.name
            );
            self.entry = description.clone();
            let fixed = self.fixed_slots(request, version);
            let (slots, _) = self.message_slots(reply, version);
            entries.push(self.entry_of(
              description,
              request,
              ("request", fixed),
              json!({ "message": slots }),
              self.source(operation, "receive", &request.name, Some(&reply.name)),
            ));
          }
        }
      }
    }
    self.entry.clear();
    entries
  }

  fn source(&self, operation: &Operation, action: &str, message: &str, reply: Option<&str>) -> Value {
    let mut source = Map::new();
    source.insert(
      "asyncapi".to_string(),
      Value::from(format!("{action} {}", operation.address)),
    );
    if let Some(id) = &operation.id {
      source.insert("operation-id".to_string(), Value::from(id.as_str()));
    }
    source.insert("message".to_string(), Value::from(message));
    if let Some(reply) = reply {
      source.insert("reply".to_string(), Value::from(reply));
    }
    Value::Object(source)
  }

  /// One entry, its selector over `part`'s fixed values, or none, which is reported.
  fn entry_of(
    &mut self,
    description: String,
    selected_by: &Message,
    (part, fixed): (&str, Map<String, Value>),
    parts: Value,
    source: Value,
  ) -> Entry {
    let mut entry = Map::new();
    entry.insert("description".to_string(), Value::from(description.as_str()));
    if fixed.is_empty() {
      let which = if part == "request" { "request" } else { "message" };
      self.lose(
        "selector-empty",
        &selected_by.pointer,
        format!(
          "the {which} fixes no header or payload member, so the entry selects no consumer \
           interaction, and is matched only by its description, '{description}'"
        ),
      );
    } else {
      entry.insert(
        "selector".to_string(),
        json!({ part: Value::Object(fixed.clone()) }),
      );
    }
    entry.insert("source".to_string(), source);
    entry.insert("parts".to_string(), parts);
    let mut flat = Vec::new();
    for (slot, shape) in &fixed {
      flatten(shape, &format!("{part}.{slot}"), &mut flat);
    }
    Entry {
      description,
      pointer: selected_by.pointer.clone(),
      fixed: flat,
      json: Value::Object(entry),
    }
  }

  /// A message's `metadata` and `body`, and the values it fixes in each.
  fn message_slots(&mut self, message: &Message, version: Version) -> (Value, Map<String, Value>) {
    let content_type = self.content_type(message);
    let mut slots = Map::new();

    let mut metadata = match self.message_headers(message, version) {
      Ok(Some((schema, pointer))) => Some(self.schema(&schema, &pointer, 0)),
      Ok(None) => None,
      Err((format, pointer)) => {
        self.lose(
          "schema-format-unread",
          &pointer,
          format!("the headers' schema is written in {format}, which is not read; no header is claimed"),
        );
        None
      }
    };
    // Pact's message model reads a message's type from its metadata's `contentType` (ADR 0067
    // decision 3), which is where a consumer's contract records it.
    if let Some(content_type) = &content_type {
      let claim = json!({ "example": content_type, "shape": "http:media-type", "type": content_type });
      match &mut metadata {
        None => metadata = Some(json!({ "members": { "contentType": claim }, "shape": "object" })),
        Some(shape) => {
          if let Some(members) = shape
            .get_mut("members")
            .and_then(Value::as_object_mut)
            .filter(|members| !members.contains_key("contentType"))
          {
            members.insert("contentType".to_string(), claim);
          }
        }
      }
    }
    if let Some(metadata) = metadata {
      slots.insert("metadata".to_string(), metadata);
    }

    match self.payload(message, version, content_type.as_deref()) {
      Payload::Schema(schema, pointer) => {
        if content_type.is_none() {
          self.lose(
            "content-type-unstated",
            &message.pointer,
            "the message names no contentType and the document no defaultContentType, so its \
             payload is read as JSON",
          );
        }
        slots.insert("body".to_string(), self.schema(&schema, &pointer, 0));
      }
      Payload::Absent => self.lose(
        "untyped-schema",
        &message.pointer,
        "the message has no payload schema, so no body is claimed",
      ),
      Payload::Format(format, pointer) => self.lose(
        "schema-format-unread",
        &pointer,
        format!(
          "the payload's schema is written in {format}, which is not read until a content \
           component for it can decode the body; no body is claimed"
        ),
      ),
      Payload::NotJson(content_type) => self.lose(
        "body-not-json",
        &message.pointer,
        format!("the body is {content_type}, and only a JSON body is derived; no body is claimed"),
      ),
    }

    let fixed = self.fixed_slots(message, version);
    (Value::Object(slots), fixed)
  }

  /// The values a message's headers and JSON payload fix, by slot.
  fn fixed_slots(&mut self, message: &Message, version: Version) -> Map<String, Value> {
    let mut fixed = Map::new();
    if let Ok(Some((schema, _))) = self.message_headers(message, version)
      && let Some(values) = self.fixed(&schema, 0)
    {
      fixed.insert("metadata".to_string(), values);
    }
    let content_type = self.content_type(message);
    if let Payload::Schema(schema, _) = self.payload(message, version, content_type.as_deref())
      && let Some(values) = self.fixed(&schema, 0)
    {
      fixed.insert("body".to_string(), values);
    }
    fixed
  }

  /// The message's own content type, else the document's default.
  fn content_type(&self, message: &Message) -> Option<String> {
    message
      .object
      .get("contentType")
      .or_else(|| self.document.get("defaultContentType"))
      .and_then(Value::as_str)
      .map(str::to_string)
  }

  /// The headers' schema, or the format it is written in when the mapping does not read it.
  fn message_headers(
    &self,
    message: &Message,
    version: Version,
  ) -> Result<Option<(Value, String)>, (String, String)> {
    let Some(headers) = message.object.get("headers") else {
      return Ok(None);
    };
    let pointer = format!("{}/headers", message.pointer);
    match multi_format(headers, version) {
      Some((format, schema)) if is_json_schema(format) => {
        Ok(Some((schema.clone(), format!("{pointer}/schema"))))
      }
      Some((format, _)) => Err((format.to_string(), pointer)),
      None => Ok(Some((headers.clone(), pointer))),
    }
  }

  fn payload(&self, message: &Message, version: Version, content_type: Option<&str>) -> Payload {
    let Some(payload) = message.object.get("payload") else {
      return Payload::Absent;
    };
    let pointer = format!("{}/payload", message.pointer);
    let (schema, pointer) = match multi_format(payload, version) {
      Some((format, schema)) if is_json_schema(format) => (schema.clone(), format!("{pointer}/schema")),
      Some((format, _)) => return Payload::Format(format.to_string(), pointer),
      None => {
        // 2.x names the payload's format on the message.
        if let Some(format) = message.object.get("schemaFormat").and_then(Value::as_str)
          && !is_json_schema(format)
        {
          return Payload::Format(format.to_string(), pointer);
        }
        (payload.clone(), pointer)
      }
    };
    match content_type {
      Some(content_type) if !is_json(content_type) => Payload::NotJson(content_type.to_string()),
      _ => Payload::Schema(schema, pointer),
    }
  }

  /// The values a schema fixes, as a selector writes them: `equality` where it allows one value
  /// (`const`, or an `enum` of one), inside the objects that lead to it. Arrays and unions are not
  /// searched, since which element or alternative a consumer's message carries is not fixed.
  fn fixed(&self, schema: &Value, depth: usize) -> Option<Value> {
    if depth > MAX_FIXED_DEPTH {
      return None;
    }
    let schema = self.dereferenced(schema)?;
    if let Some(constant) = schema.get("const") {
      return Some(json!({ "example": constant, "shape": "equality" }));
    }
    if let Some([only]) = schema.get("enum").and_then(Value::as_array).map(Vec::as_slice) {
      return Some(json!({ "example": only, "shape": "equality" }));
    }
    let mut properties = Map::new();
    self.properties(&schema, depth, &mut properties);
    let mut members = Map::new();
    for (name, property) in &properties {
      if let Some(value) = self.fixed(property, depth + 1) {
        members.insert(name.clone(), value);
      }
    }
    (!members.is_empty()).then(|| json!({ "members": Value::Object(members), "shape": "object" }))
  }

  /// An object's properties, its `allOf` branches' included.
  fn properties(&self, schema: &Value, depth: usize, properties: &mut Map<String, Value>) {
    if depth > MAX_FIXED_DEPTH {
      return;
    }
    let Some(schema) = self.dereferenced(schema) else {
      return;
    };
    for branch in schema
      .get("allOf")
      .and_then(Value::as_array)
      .into_iter()
      .flatten()
    {
      self.properties(branch, depth + 1, properties);
    }
    for (name, property) in schema
      .get("properties")
      .and_then(Value::as_object)
      .into_iter()
      .flatten()
    {
      properties.insert(name.clone(), property.clone());
    }
  }

  /// A schema with its references followed, silently: the mapping reports one that is not.
  fn dereferenced(&self, schema: &Value) -> Option<Value> {
    let mut schema = schema.clone();
    for _ in 0..MAX_REFERENCES {
      match schema.get("$ref").and_then(Value::as_str) {
        Some(reference) => schema = self.lookup(reference)?.clone(),
        None => return Some(schema),
      }
    }
    None
  }

  /// Every pair of entries one message could be selected by: `janus check` chooses neither, so the
  /// consumer's interaction is `not-published` (subsumption-check spec §2.2).
  fn ties(&mut self, entries: &[Entry]) {
    for (index, entry) in entries.iter().enumerate() {
      for (other_index, other) in entries.iter().enumerate() {
        if index == other_index || entry.fixed.is_empty() || other.fixed.is_empty() {
          continue;
        }
        let conflict = entry.fixed.iter().any(|(path, value)| {
          other
            .fixed
            .iter()
            .any(|(other_path, other_value)| path == other_path && value != other_value)
        });
        if !conflict {
          self.entry = entry.description.clone();
          self.lose(
            "selector-ambiguous",
            &entry.pointer,
            format!(
              "a message that carries what this entry's selector fixes may also be selected by \
               '{}', and janus check chooses neither, so its consumers' interactions are not-published",
              other.description
            ),
          );
        }
      }
    }
    self.entry.clear();
  }
}

/// AsyncAPI 3.0's Multi Format Schema Object: a schema with the format it is written in.
fn multi_format(value: &Value, version: Version) -> Option<(&str, &Value)> {
  if version != Version::Three {
    return None;
  }
  let format = value.get("schemaFormat").and_then(Value::as_str)?;
  Some((format, value.get("schema").unwrap_or(&Value::Null)))
}

/// A schema format the JSON Schema mapping reads: AsyncAPI's own, JSON Schema's, and OpenAPI 3.0's.
fn is_json_schema(format: &str) -> bool {
  let essence = format
    .split(';')
    .next()
    .unwrap_or_default()
    .trim()
    .to_ascii_lowercase();
  essence.starts_with("application/vnd.aai.asyncapi")
    || essence == "application/schema+json"
    || essence == "application/schema+yaml"
    || essence.starts_with("application/vnd.oai.openapi")
}

/// A 3.0 channel's address, else its id when the address is only known at run time.
fn address_3(channel: &Value, pointer: &str) -> String {
  channel
    .get("address")
    .and_then(Value::as_str)
    .map(str::to_string)
    .unwrap_or_else(|| pointer.rsplit('/').next().map(unescape).unwrap_or_default())
}

/// RFC 7386's merge patch, which AsyncAPI applies traits with.
fn merge_patch(target: &mut Value, patch: &Value) {
  let Some(patch) = patch.as_object() else {
    *target = patch.clone();
    return;
  };
  if !target.is_object() {
    *target = json!({});
  }
  let Some(target) = target.as_object_mut() else {
    return;
  };
  for (key, value) in patch {
    if value.is_null() {
      target.remove(key);
    } else {
      merge_patch(target.entry(key.clone()).or_insert(Value::Null), value);
    }
  }
}

/// A selector's fixed values by where they are: `message.metadata.queue`.
fn flatten(shape: &Value, path: &str, into: &mut Vec<(String, Value)>) {
  match shape.get("shape").and_then(Value::as_str) {
    Some("equality") => into.push((path.to_string(), shape["example"].clone())),
    Some("object") => {
      for (name, member) in shape
        .get("members")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
      {
        flatten(member, &format!("{path}.{name}"), into);
      }
    }
    _ => {}
  }
}

/// A JSON pointer's reference token, unescaped (RFC 6901).
fn unescape(token: &str) -> String {
  token.replace("~1", "/").replace("~0", "~")
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn a_trait_is_merged_as_a_merge_patch() {
    let mut message = json!({ "headers": { "type": "object" }, "contentType": "application/json" });
    merge_patch(
      &mut message,
      &json!({ "contentType": "application/avro", "headers": { "required": ["a"] } }),
    );
    assert_eq!(
      message,
      json!({ "headers": { "type": "object", "required": ["a"] }, "contentType": "application/avro" })
    );
  }

  #[test]
  fn json_schema_formats_are_read_and_others_are_not() {
    assert!(is_json_schema("application/vnd.aai.asyncapi+json;version=3.0.0"));
    assert!(is_json_schema("application/schema+yaml;version=draft-07"));
    assert!(is_json_schema("application/vnd.oai.openapi;version=3.0.0"));
    assert!(!is_json_schema("application/vnd.apache.avro+json;version=1.9.0"));
    assert!(!is_json_schema("application/raml+yaml;version=1.0"));
  }

  #[test]
  fn an_address_only_known_at_run_time_is_its_channels_id() {
    assert_eq!(
      address_3(&json!({ "address": "orders.placed" }), "/channels/x"),
      "orders.placed"
    );
    assert_eq!(
      address_3(&json!({ "address": null }), "/channels/user~1replies"),
      "user/replies"
    );
  }
}
