//! The FFI's builder, held to the FFI: each pact-js example's calls, written as pact-js's builders
//! make them, build the interaction pact-js's pact holds. The pacts are S1.1's fixtures, which
//! pact-js wrote through `pact_ffi`.

use super::*;
use pretty_assertions::assert_eq;

/// The v4 form of the interaction `description` in a pact-js example's pact.
fn recorded(example: &str, description: &str) -> Value {
  let path = format!(
    "{}/../../corpora/verifier/examples/pact-js/{example}/pact.json",
    env!("CARGO_MANIFEST_DIR")
  );
  let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"));
  let pact = crate::legacy::pact::read(&path, &serde_json::from_str(&text).unwrap()).unwrap();
  let v4 = pact.as_v4_pact().unwrap();
  let interaction = v4
    .interactions
    .iter()
    .find(|interaction| interaction.description() == description)
    .unwrap_or_else(|| panic!("{example} has no '{description}'"));
  comparable(interaction.as_v4_http().unwrap().to_json())
}

/// What the FFI decides: the description, states, request and response. A body is compared as the
/// document it holds, without its label: a v3 pact read as v4 carries a JSON body as base64
/// labelled `*/*`, and its `Content-Type` header is compared anyway.
fn comparable(json: Value) -> Value {
  let part = |part: &Value| {
    let mut part = part.clone();
    if let Some(body) = part.get_mut("body").and_then(Value::as_object_mut) {
      let content = match (body.get("encoded"), body.get("content")) {
        (Some(Value::String(encoding)), Some(Value::String(text))) if encoding == "base64" => {
          let bytes = BASE64.decode(text).unwrap();
          serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(text.clone()))
        }
        (_, Some(content)) => content.clone(),
        _ => Value::Null,
      };
      body.insert("content".to_string(), content);
      body.remove("encoded");
      body.remove("contentType");
    }
    part
  };
  json!({
    "description": json["description"],
    "providerStates": json["providerStates"],
    "request": part(&json["request"]),
    "response": part(&json["response"]),
  })
}

fn built(steps: Value) -> Value {
  built_as("V4", steps)
}

fn built_as(specification: &str, steps: Value) -> Value {
  let interaction = build(&json!({ "specification": specification, "steps": steps }))
    .unwrap_or_else(|problems| panic!("{problems:?}"));
  comparable(interaction.to_json())
}

fn like(value: Value) -> Value {
  json!({ "pact:matcher:type": "type", "value": value })
}

fn integer(value: i64) -> Value {
  json!({ "pact:matcher:type": "integer", "value": value })
}

fn decimal(value: f64) -> Value {
  json!({ "pact:matcher:type": "decimal", "value": value })
}

fn each_like(value: Value) -> Value {
  json!({ "pact:matcher:type": "type", "min": 1, "value": [value] })
}

/// A V4 builder's `.withRequest(method, path, b => b.headers({ Accept }))`.
fn get_accepting(path: &str, accept: &str) -> Vec<Value> {
  vec![
    json!({ "call": "with_request", "method": "GET", "path": path }),
    json!({ "call": "with_header_v2", "part": "request", "name": "Accept", "index": 0, "value": accept }),
  ]
}

/// A V4 builder's `.willRespondWith(status, b => { b.headers(…); b.jsonBody(body) })`.
fn responding(status: u16, content_type: &str, body: Value) -> Vec<Value> {
  vec![
    json!({ "call": "response_status_v2", "status": status.to_string() }),
    json!({ "call": "with_header_v2", "part": "response", "name": "Content-Type", "index": 0, "value": content_type }),
    json!({ "call": "with_body", "part": "response", "content_type": "application/json", "body": body }),
  ]
}

fn steps(parts: Vec<Vec<Value>>) -> Value {
  Value::Array(parts.into_iter().flatten().collect())
}

fn user() -> Value {
  json!({ "id": integer(1), "name": like(json!("Alice")), "email": like(json!("alice@example.com")) })
}

#[test]
fn pact_js_http_example() {
  assert_eq!(
    built(steps(vec![
      vec![
        json!({ "call": "given", "description": "a user with ID 1 exists" }),
        json!({ "call": "upon_receiving", "description": "a GET request for user 1" }),
      ],
      get_accepting("/users/1", "application/json"),
      responding(200, "application/json", like(user())),
    ])),
    recorded("http", "a GET request for user 1")
  );
  assert_eq!(
    built(steps(vec![
      vec![
        json!({ "call": "given", "description": "users exist" }),
        json!({ "call": "upon_receiving", "description": "a GET request for all users" }),
      ],
      get_accepting("/users", "application/json"),
      responding(200, "application/json", each_like(user())),
    ])),
    recorded("http", "a GET request for all users")
  );
}

fn product() -> Value {
  json!({ "id": integer(1), "name": like(json!("Widget")), "price": decimal(9.99) })
}

#[test]
fn pact_js_matchers_example() {
  let product_body = json!({
    "id": integer(1),
    "name": like(json!("Widget")),
    "price": decimal(9.99),
    "category": { "pact:matcher:type": "regex", "regex": "^[a-z-]+$", "value": "electronics" },
    "createdAt": {
      "pact:matcher:type": "timestamp", "format": "yyyy-MM-dd'T'HH:mm:ss.SSS'Z'",
      "value": "2024-01-01T00:00:00.000Z"
    },
    "tags": each_like(json!("featured")),
  });
  assert_eq!(
    built(steps(vec![
      vec![
        json!({ "call": "given", "description": "a product exists" }),
        json!({ "call": "upon_receiving", "description": "a GET request for a product" }),
      ],
      get_accepting("/products/1", "application/json"),
      responding(200, "application/json", product_body),
    ])),
    recorded("matchers", "a GET request for a product")
  );

  let catalog = json!({
    "pact:matcher:type": "eachKey",
    "rules": [ { "pact:matcher:type": "regex", "regex": "^[A-Z]+-\\d{3}$", "value": "WIDGET-001" } ],
    "value": { "WIDGET-001": { "name": like(json!("Widget")), "price": decimal(9.99) } }
  });
  assert_eq!(
    built(steps(vec![
      vec![
        json!({ "call": "given", "description": "the catalog is populated" }),
        json!({ "call": "upon_receiving", "description": "a GET request for the product catalog" }),
      ],
      get_accepting("/catalog", "application/json"),
      responding(200, "application/json", catalog),
    ])),
    recorded("matchers", "a GET request for the product catalog")
  );

  // `matchStatus(200, Success)`: the reified status, then the status rule as matching rules.
  let mut flexible = steps(vec![
    vec![
      json!({ "call": "given", "description": "products exist" }),
      json!({ "call": "upon_receiving", "description": "a GET request for all products with flexible status matching" }),
    ],
    get_accepting("/products", "application/json"),
    responding(200, "application/json", each_like(product())),
  ]);
  flexible.as_array_mut().unwrap().insert(
    4,
    json!({ "call": "with_matching_rules", "part": "response",
            "rules": { "status": { "$": { "matchers": [ { "match": "statusCode", "status": "success" } ] } } } }),
  );
  assert_eq!(
    built(flexible),
    recorded(
      "matchers",
      "a GET request for all products with flexible status matching"
    )
  );
}

#[test]
fn pact_js_provider_state_example_on_pact_v3() {
  // PactV3: the states follow the description, and the body follows the response's headers.
  assert_eq!(
    built_as(
      "V3",
      json!([
        { "call": "upon_receiving", "description": "a GET request for account ACC-001" },
        { "call": "given_with_params", "description": "an account with number {accountNumber} exists",
          "params": { "accountNumber": "ACC-001" } },
        { "call": "with_request", "method": "GET", "path": "/accounts/ACC-001" },
        { "call": "with_header_v2", "part": "request", "name": "Accept", "index": 0, "value": "application/json" },
        { "call": "response_status_v2", "status": "200" },
        { "call": "with_header_v2", "part": "response", "name": "Content-Type", "index": 0, "value": "application/json" },
        { "call": "with_body", "part": "response", "content_type": "application/json", "body": {
            "id": { "pact:matcher:type": "type", "pact:generator:type": "ProviderState",
                    "expression": "${accountId}", "value": 42 },
            "accountNumber": like(json!("ACC-001")),
            "ownerName": like(json!("Jane Smith")),
            "balance": integer(1000)
        } }
      ])
    ),
    recorded("provider-state", "a GET request for account ACC-001")
  );
}

#[test]
fn pact_js_graphql_example() {
  let query = "\n        query HelloQuery {\n          hello\n        }\n      ";
  assert_eq!(
    built(json!([
      { "call": "given", "description": "the hello service is available" },
      { "call": "upon_receiving", "description": "a HelloQuery request" },
      { "call": "with_request", "method": "POST", "path": "/graphql" },
      { "call": "with_body", "part": "request", "content_type": "application/json", "body": {
          "operationName": "HelloQuery",
          "query": { "pact:matcher:type": "regex", "regex": "\\s*query\\s*HelloQuery\\s*\\{\\s*hello\\s*\\}\\s*", "value": query },
          "variables": { "name": "World" }
      } },
      { "call": "response_status_v2", "status": "200" },
      { "call": "with_header_v2", "part": "response", "name": "Content-Type", "index": 0, "value": "application/json; charset=utf-8" },
      { "call": "with_body", "part": "response", "content_type": "application/json",
        "body": { "data": { "hello": like(json!("Hello, World!")) } } }
    ])),
    recorded("graphql", "a HelloQuery request")
  );
}

#[test]
fn pact_js_xml_example() {
  let text = |value: &str| json!({ "content": value, "matcher": like(json!(value)) });
  // `XmlBuilder`'s JSON: a `Map` of attributes stringifies as `{}`, so `id="1"` is lost, as it is
  // in the pact pact-js wrote.
  let document = json!({ "root": { "name": "catalogue", "children": [
    { "pact:matcher:type": "type", "examples": 1, "value": { "name": "book", "attributes": {}, "children": [
      { "name": "title", "attributes": {}, "children": [ text("The Pragmatic Programmer") ] },
      { "name": "author", "attributes": {}, "children": [ text("David Thomas") ] }
    ] } }
  ] } });
  assert_eq!(
    built(steps(vec![
      vec![
        json!({ "call": "given", "description": "the catalogue has books" }),
        json!({ "call": "upon_receiving", "description": "a GET request for the catalogue in XML" }),
      ],
      get_accepting("/catalogue", "application/xml"),
      vec![
        json!({ "call": "response_status_v2", "status": "200" }),
        json!({ "call": "with_header_v2", "part": "response", "name": "Content-Type", "index": 0,
                "value": "application/xml; charset=utf-8" }),
        // `xmlBody` passes the builder's JSON text.
        json!({ "call": "with_body", "part": "response", "content_type": "application/xml", "body": document.to_string() }),
      ],
    ])),
    recorded("xml", "a GET request for the catalogue in XML")
  );
}

const SAMPLE: &[u8] = b"Sample file for multipart upload testing.\nThis file is a test fixture.\n";

#[test]
fn pact_js_multipart_example() {
  let upload = |boundary: Option<&str>| {
    json!([
      { "call": "given", "description": "the upload service is ready" },
      { "call": "upon_receiving", "description": "a multipart file upload" },
      { "call": "with_request", "method": "POST", "path": "/upload" },
      { "call": "with_multipart_file_v2", "part": "request", "content_type": "text/plain",
        "file": "/somewhere/fixtures/sample.txt", "contents": BASE64.encode(SAMPLE),
        "part_name": "file", "boundary": boundary },
      { "call": "response_status_v2", "status": "200" },
      { "call": "with_header_v2", "part": "response", "name": "Content-Type", "index": 0, "value": "application/json" },
      { "call": "with_body", "part": "response", "content_type": "application/json", "body": { "success": true } }
    ])
  };
  // pact_ffi drew this boundary at random when pact-js recorded the pact.
  assert_eq!(
    built(upload(Some("U9iXwlWODmM9xpRU"))),
    recorded("multipart", "a multipart file upload")
  );

  // Without one, the boundary is derived from the part: the same on every run (ADR 0066 decision 7).
  let derived = built(upload(None));
  assert_eq!(derived, built(upload(None)));
  let header = derived["request"]["headers"]["Content-Type"][0]
    .as_str()
    .unwrap()
    .to_string();
  let boundary = header.strip_prefix("multipart/form-data; boundary=").unwrap();
  let body = String::from_utf8(
    BASE64
      .decode(derived["request"]["body"]["content"].as_str().unwrap())
      .unwrap(),
  )
  .unwrap();
  assert!(body.starts_with(&format!("--{boundary}\r\n")), "{body}");
  assert!(body.ends_with(&format!("--{boundary}--\r\n")), "{body}");
}

#[test]
fn a_second_file_joins_the_first_multipart_body() {
  let file = |name: &str, contents: &str| {
    json!({ "call": "with_multipart_file_v2", "part": "request", "content_type": "text/plain",
            "file": format!("{name}.txt"), "contents": BASE64.encode(contents), "part_name": name, "boundary": "b" })
  };
  let built = built(json!([
    { "call": "upon_receiving", "description": "two files" },
    { "call": "with_request", "method": "POST", "path": "/upload" },
    file("one", "1"),
    file("two", "2")
  ]));
  let body = String::from_utf8(
    BASE64
      .decode(built["request"]["body"]["content"].as_str().unwrap())
      .unwrap(),
  )
  .unwrap();
  assert_eq!(
    body,
    "--b\r\nContent-Disposition: form-data; name=\"one\"; filename=\"one.txt\"\r\nContent-Type: text/plain\r\n\r\n1\r\n\
     --b\r\nContent-Disposition: form-data; name=\"two\"; filename=\"two.txt\"\r\nContent-Type: text/plain\r\n\r\n2\r\n--b--\r\n"
  );
  assert_eq!(
    built["request"]["matchingRules"]["body"]["$.two"]["matchers"][0]["match"],
    "contentType"
  );
}

#[test]
fn a_form_body_written_as_json_is_a_query_string() {
  let built = built(json!([
    { "call": "upon_receiving", "description": "a form" },
    { "call": "with_request", "method": "POST", "path": "/form" },
    { "call": "with_body", "part": "request", "content_type": "application/x-www-form-urlencoded",
      "body": { "name": like(json!("Fred")), "ids": [1, 2], "skipped": { "a": 1 } } }
  ]));
  assert_eq!(built["request"]["body"]["content"], "name=Fred&ids=1&ids=2");
  assert_eq!(
    built["request"]["matchingRules"]["body"],
    json!({ "$.name": { "combine": "AND", "matchers": [ { "match": "type" } ] } })
  );
}

#[test]
fn query_values_take_matchers_and_header_values_split_as_the_ffi_splits_them() {
  let built = built(json!([
    { "call": "upon_receiving", "description": "a query" },
    { "call": "with_request", "method": "GET", "path": "/search" },
    { "call": "with_query_parameter", "name": "q", "index": 0,
      "value": { "pact:matcher:type": "regex", "regex": "\\w+", "value": "pact" } },
    { "call": "with_query_parameter", "name": "q", "index": 1, "value": "janus" },
    { "call": "with_header_v2", "part": "request", "name": "Accept", "index": 0, "value": "application/json, text/plain" }
  ]));
  assert_eq!(built["request"]["query"], json!({ "q": ["pact", "janus"] }));
  // A v4 pact writes a query rule under the parameter's name.
  assert_eq!(
    built["request"]["matchingRules"]["query"]["q"]["matchers"][0]["regex"],
    "\\w+"
  );
  assert_eq!(
    built["request"]["headers"]["Accept"],
    json!(["application/json", "text/plain"])
  );
}

#[test]
fn a_binary_body_has_a_content_type_rule_from_v3() {
  let steps = |specification| {
    built_as(
      specification,
      json!([
        { "call": "upon_receiving", "description": "an image" },
        { "call": "with_request", "method": "GET", "path": "/image" },
        { "call": "with_binary_file", "part": "response", "content_type": "image/png", "body": BASE64.encode([0x89, b'P', b'N', b'G']) }
      ]),
    )
  };
  assert_eq!(
    steps("V3")["response"]["matchingRules"]["body"]["$"]["matchers"][0]["match"],
    "contentType"
  );
  assert_eq!(steps("V2")["response"]["matchingRules"], Value::Null);
  assert_eq!(
    steps("V3")["response"]["headers"]["Content-Type"],
    json!(["image/png"])
  );
}

#[test]
fn each_problem_points_at_its_step() {
  let problems = build(&json!({ "steps": [
    { "call": "upon_receiving", "description": "broken" },
    { "call": "with_plugin_contents" },
    { "call": "with_header_v2", "part": "sideways", "name": "A", "value": "b" },
    { "call": "with_body", "part": "response", "content_type": "application/json",
      "body": { "id": { "pact:matcher:type": "regex", "value": "1" } } },
    { "call": "with_body", "part": "response", "content_type": "application/xml", "body": "{\"root\": 1}" }
  ] }))
  .err()
  .unwrap();
  let pointers: Vec<&str> = problems.iter().map(|p| p.pointer.as_str()).collect();
  assert_eq!(
    pointers,
    [
      "/pact/steps/1/call",
      "/pact/steps/2/part",
      "/pact/steps/3",
      "/pact/steps/4"
    ]
  );
}

#[test]
fn the_built_interaction_converts_as_upgrade_converts_the_pact() {
  let converted = interaction(&json!({ "steps": steps(vec![
    vec![
      json!({ "call": "given", "description": "a user with ID 1 exists" }),
      json!({ "call": "upon_receiving", "description": "a GET request for user 1" }),
    ],
    get_accepting("/users/1", "application/json"),
    responding(200, "application/json", like(user())),
  ]) }))
  .unwrap_or_else(|problems| panic!("{problems:?}"));

  let path = format!(
    "{}/../../corpora/verifier/examples/pact-js/http/pact.json",
    env!("CARGO_MANIFEST_DIR")
  );
  let pact = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
  let upgraded = upgrade::pact(&path, &pact).unwrap();
  let expected = upgraded
    .contract
    .interactions
    .iter()
    .find(|interaction| interaction.description == "a GET request for user 1")
    .unwrap();
  assert_eq!(converted.spec, expected.spec_document());
  // Pointers into the built interaction, not into a one-interaction pact nobody wrote.
  assert!(
    converted
      .findings
      .iter()
      .all(|finding| !finding.path.starts_with("/interactions"))
  );
}
