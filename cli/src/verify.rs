//! `janus verify` (plan task 5.5): source documents in, a provider under test, a verdict out.
//!
//! **Everything this command knows about a run, it learns from events.** It starts the run with
//! one `verification/verify` frame and then polls the stream until the terminal event (spec §9.2:
//! termination is structural, so this loop reads `last` and never a kind vocabulary). There is no
//! second channel and no Rust callback — which is what makes the rendering below something an SDK
//! could reproduce exactly, and why the RFC insists hook activity and per-variant results are
//! events rather than log lines.
//!
//! Sources are read as *documents*, never sniffed: a Janus contract identifies itself (ADR 0011)
//! and a v1–v4 pact is read as one (plan task 5.4). Both are handed to the engine untouched, which
//! is why `janus verify` needs no flag saying which kind of file it was given.

use crate::Outcome;
use crate::args::{Output, TransportEntry, VerifyArgs};
use crate::engine;
use crate::error::{self, Error};
use crate::highlight;
use crate::io;
use clap::error::ErrorKind;
use serde_json::{Value, json};

/// How long one `events/poll` may wait for something to happen. An empty answer only means
/// nothing did within it: the run is over when its terminal event arrives, and only then (protocol
/// spec §9.6 — a run that cannot complete still sends one).
const POLL_WAIT_MS: u64 = 10_000;

pub fn run(args: &VerifyArgs) -> Result<Outcome, Error> {
  // Where the provider is, before anything is read: a usage error, not a file's.
  provider_url(args)?;
  let (source, files) = source(args)?;

  let hooks = match &args.config {
    None => None,
    Some(path) => Some(
      pact_janus_hooks_host::loader::load(path)
        .map_err(|err| format!("reading {}: {err}", path.display()))?,
    ),
  };

  let mut options = json!({});
  if let Some(filter) = interaction_filter(args) {
    options["filter"] = filter;
  }
  if !args.variants.is_empty() {
    options["variants"] = json!(args.variants);
  }
  if args.explain_failures {
    options["executed-plan"] = json!("on-failure");
  }
  if args.exit_on_first_error {
    options["stop-on-first-failure"] = json!(true);
  }
  if args.publish {
    // `requires_all` guarantees the version; the branch and tags are the provider version's.
    let mut publish = json!({ "provider-version": args.provider_version });
    if let Some(branch) = &args.provider_branch {
      publish["branch"] = json!(branch);
    }
    if !args.provider_tags.is_empty() {
      publish["tags"] = json!(args.provider_tags);
    }
    if let Some(build_url) = &args.build_url {
      publish["build-url"] = json!(build_url);
    }
    options["publish"] = publish;
  }

  let provider = provider_options(args)?;
  let mut target = json!({
    "transports": [{
      "transport": transport_kind(args),
      "options": provider,
    }]
  });
  let mut hooks = hooks.unwrap_or_else(|| json!({ "version": 1 }));
  // One file, two members, two owners (lifecycle-hooks spec §6.1): the components are design
  // 2.6's and ride beside the hooks, not inside them.
  if let Some(components) = hooks
    .as_object_mut()
    .and_then(|config| config.remove("components"))
  {
    target["components"] = components;
  }
  add_state_change(&mut hooks, args, &target["transports"][0]["options"])?;
  add_message_producer(&mut hooks, args, &target["transports"][0]["options"])?;
  target["hooks"] = hooks;

  let mut engine = engine::start()?;
  let started = engine::call_with_files(
    &mut engine,
    "verification/verify",
    json!({
      "source": source,
      "target": target,
      "options": options,
    }),
    &files,
  )?;
  let stream = started["stream"].as_str().unwrap_or_default().to_string();

  let text = args.output == Output::Text;
  let mut reporter = Reporter::default();
  let summary = loop {
    let polled = engine::call(
      &mut engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": POLL_WAIT_MS }),
    )?;
    let events = polled["events"].as_array().cloned().unwrap_or_default();
    if events.is_empty() {
      tracing::debug!(stream, "no events within {POLL_WAIT_MS}ms; still waiting");
      continue;
    }
    let mut last = None;
    for event in events {
      if text {
        reporter.report(&event);
      }
      if event["last"] == json!(true) {
        last = Some(event["payload"].clone());
      }
    }
    if let Some(summary) = last {
      break summary;
    }
  };

  // A run the engine ended before it had anything to count — a broker or a URL it could not fetch,
  // no pacts selected, a variant nothing records — did not run, and says why as any command that
  // could not run does: exit 2, not an answer about the provider (staged plan S1.29).
  let tallied = summary["variants"].is_object();
  if text && tallied {
    outln!();
    outln!("{}", render_summary(&summary));
  } else if !text {
    outln!("{}", crate::output::pretty(&summary));
  }
  if !tallied {
    return Err(Error::Failed(
      match summary.get("error").filter(|e| !e.is_null()) {
        Some(error) => engine::render_error(error),
        None => "the run ended before it verified anything".to_string(),
      },
    ));
  }
  // A filtered run did what it was asked: its summary says it is not a pass, and its result is not
  // published (ADR 0041 decision 5).
  Ok(match summary["status"].as_str() {
    Some("verified" | "filtered") => Outcome::Done,
    _ => Outcome::SubjectFailed,
  })
}

/// `options.filter` from today's `--filter-*` options (ADR 0053), when any is given.
fn interaction_filter(args: &VerifyArgs) -> Option<Value> {
  let mut filter = serde_json::Map::new();
  if let Some(description) = &args.filter_description {
    filter.insert("description".to_string(), json!(description));
  }
  if let Some(state) = &args.filter_state {
    filter.insert("state".to_string(), json!(state));
  }
  if args.filter_no_state {
    filter.insert("no-state".to_string(), json!(true));
  }
  if !args.filter_consumer.is_empty() {
    filter.insert("consumers".to_string(), json!(args.filter_consumer));
  }
  (!filter.is_empty()).then_some(Value::Object(filter))
}

/// The HTTP transport's drive options (component-interfaces spec §5.6) from the command line:
/// `base-url` with `--base-path` after it, `--header`s, and the HTTP client options. Certificates
/// and keys are read here and passed as text (ADR 0035 decision 3): the engine reads no file for
/// them. Only what was asked for is sent, so the engine's defaults are the defaults.
fn provider_options(args: &VerifyArgs) -> Result<Value, Error> {
  // An `http` entry in `--transports` is where HTTP interactions go, as today: its port and its
  // path, on the provider URL's host, in place of the URL and `--base-path`.
  let base_url = match args.transports.iter().find(|entry| entry.name == "http") {
    Some(entry) => transport_url(args, entry)?,
    None => provider_base_url(args)?,
  };
  let mut options = json!({ "base-url": base_url });
  if !args.headers.is_empty() || !args.custom_provider_header.is_empty() {
    // A later --header of the same name replaces an earlier one, as today's verifier's map does.
    let mut headers = serde_json::Map::new();
    for (name, value) in args.custom_provider_header.iter().chain(&args.headers) {
      headers.retain(|existing, _| !existing.eq_ignore_ascii_case(name));
      headers.insert(name.clone(), json!(value));
    }
    options["headers"] = Value::Object(headers);
  }

  let pem = |flag: &str, path: &std::path::Path| {
    std::fs::read_to_string(path)
      .map_err(|err| Error::from(format!("{flag} {}: {}", path.display(), io::io_error(&err))))
  };
  let mut client = serde_json::Map::new();
  let mut tls = serde_json::Map::new();
  if !args.ca_certs.is_empty() {
    let certificates = args
      .ca_certs
      .iter()
      .map(|path| pem("--ca-cert", path))
      .collect::<Result<Vec<_>, _>>()?;
    tls.insert("ca-certificates".to_string(), json!(certificates));
  }
  if let (Some(certificate), Some(key)) = (&args.client_cert, &args.client_key) {
    tls.insert(
      "client-certificate".to_string(),
      json!({ "certificate": pem("--client-cert", certificate)?, "key": pem("--client-key", key)? }),
    );
  }
  if args.disable_ssl_verification {
    tls.insert("insecure".to_string(), json!(true));
  }
  if !tls.is_empty() {
    client.insert("tls".to_string(), Value::Object(tls));
  }
  if let Some(proxy) = &args.proxy {
    let mut proxy = json!({ "kind": "url", "url": proxy });
    if !args.no_proxy.is_empty() {
      proxy["no-proxy"] = json!(args.no_proxy);
    }
    client.insert("proxy".to_string(), proxy);
  }
  let mut timeouts = serde_json::Map::new();
  if let Some(ms) = args.request_timeout {
    timeouts.insert("request-ms".to_string(), json!(ms));
  }
  if let Some(ms) = args.connect_timeout {
    timeouts.insert("connect-ms".to_string(), json!(ms));
  }
  if !timeouts.is_empty() {
    client.insert("timeouts".to_string(), Value::Object(timeouts));
  }
  if let Some(version) = args.http_version {
    client.insert("http-version".to_string(), json!(version.as_str()));
  }
  if !client.is_empty() {
    options["client"] = Value::Object(client);
  }
  Ok(options)
}

/// The provider URL with `--base-path` after it: where HTTP interactions go, and where a message is
/// asked for when no `--transports` entry says otherwise.
fn provider_base_url(args: &VerifyArgs) -> Result<String, Error> {
  let mut base_url = provider_url(args)?.trim_end_matches('/').to_string();
  if let Some(path) = args.base_path.as_deref().map(|path| path.trim_matches('/'))
    && !path.is_empty()
  {
    base_url.push('/');
    base_url.push_str(path);
  }
  Ok(base_url)
}

/// Where the provider is: `--provider-url`, or today's `--hostname`, `--port` and `--transport
/// https` (staged plan S1.13), which default to localhost and the scheme's port, as today's do.
fn provider_url(args: &VerifyArgs) -> Result<String, Error> {
  if let Some(url) = &args.provider_url {
    return Ok(url.clone());
  }
  if args.hostname.is_none() && args.port.is_none() {
    return Err(error::usage(
      &["verify"],
      ErrorKind::MissingRequiredArgument,
      "say where the provider is: --provider-url <URL>, or --hostname and --port",
    ));
  }
  let scheme = if args.transport == "https" {
    "https"
  } else {
    "http"
  };
  let host = args.hostname.as_deref().unwrap_or("localhost");
  Ok(match args.port {
    Some(port) => format!("{scheme}://{host}:{port}"),
    None => format!("{scheme}://{host}"),
  })
}

/// The transport component to bind: `--transport`, where today's `https` is the `http` transport
/// over TLS, the scheme [`provider_url`] gives it.
fn transport_kind(args: &VerifyArgs) -> &str {
  match args.transport.as_str() {
    "https" => "http",
    kind => kind,
  }
}

/// A `--transports` entry's URL, as today's verifier makes one (`ProviderTransport::base_url`):
/// plain HTTP, the provider URL's host, the entry's port and its path, `--base-path` not included.
fn transport_url(args: &VerifyArgs, entry: &TransportEntry) -> Result<String, Error> {
  let provider_url = provider_url(args)?;
  let provider =
    url::Url::parse(&provider_url).map_err(|err| format!("--provider-url {provider_url}: {err}"))?;
  let host = provider
    .host_str()
    .ok_or_else(|| format!("--provider-url {provider_url} names no host for --transports"))?;
  Ok(format!(
    "http://{host}:{}{}",
    entry.port,
    entry.path.as_deref().unwrap_or_default()
  ))
}

/// The name of the `produce-message` hook `janus verify` adds when the configuration names none.
const MESSAGE_PRODUCER: &str = "provider-messages";

/// Unless the configuration names a `produce-message` hook, ask the provider for each message as
/// today's verifier does (staged plan S1.7): an `http` hook in `pact-message` format, POSTed to the
/// `--transports` entry the message is routed to, else the provider URL, with the `--header`s and
/// the client options the provider is reached with. A configuration's own hooks at the point replace
/// it whole, since the hooks at a point chain and a second producer would overwrite the first.
fn add_message_producer(hooks: &mut Value, args: &VerifyArgs, provider: &Value) -> Result<(), Error> {
  let configured = hooks
    .pointer("/hooks/produce-message")
    .and_then(Value::as_array)
    .is_some_and(|entries| !entries.is_empty());
  if configured {
    return Ok(());
  }
  let transports = args
    .transports
    .iter()
    .map(|entry| Ok(json!({ "name": entry.name, "url": transport_url(args, entry)? })))
    .collect::<Result<Vec<_>, Error>>()?;
  let mut run = json!({
    "kind": "http",
    "url": provider_base_url(args)?,
    "format": "pact-message",
  });
  if !transports.is_empty() {
    run["transports"] = json!(transports);
  }
  if let Some(headers) = provider.get("headers") {
    run["headers"] = headers.clone();
  }
  if let Some(client) = provider.get("client") {
    run["client"] = client.clone();
  }
  let mut entry = json!({ "name": MESSAGE_PRODUCER, "run": run, "changes": ["parts"] });
  if let Some(ms) = args.request_timeout {
    entry["timeout-ms"] = json!(ms);
  }
  let Some(config) = hooks.as_object_mut() else {
    return Err(String::from("the verifier configuration is not an object").into());
  };
  let points = config.entry("hooks").or_insert_with(|| json!({}));
  let Some(points) = points.as_object_mut() else {
    return Err(String::from("the verifier configuration's hooks are not an object").into());
  };
  points.insert("produce-message".to_string(), json!([entry]));
  Ok(())
}

/// The name of the hooks `janus verify` writes for today's state-change options.
const STATE_CHANGE: &str = "state-change-url";

/// Today's `--state-change-url`, `--state-change-as-query` and `--state-change-teardown`, written as
/// the hooks they become (staged plan S1.13, ADR 0053): an `http` hook in `pact-state-change` format
/// at `state-setup`, and at `state-teardown` too when asked, bounded by `--request-timeout` and
/// reached with the client options the provider is, as today's verifier reaches it. A configuration
/// with `state-setup` hooks of its own is refused: the states would be set up twice.
fn add_state_change(hooks: &mut Value, args: &VerifyArgs, provider: &Value) -> Result<(), Error> {
  let Some(url) = &args.state_change_url else {
    return Ok(());
  };
  let configured = hooks
    .pointer("/hooks/state-setup")
    .and_then(Value::as_array)
    .is_some_and(|entries| !entries.is_empty());
  if configured {
    return Err(error::usage(
      &["verify"],
      ErrorKind::ArgumentConflict,
      "--state-change-url and a configuration with state-setup hooks of its own would set the states up twice; say it once",
    ));
  }
  let format = match args.state_change_as_query {
    true => "pact-state-change-query",
    false => "pact-state-change",
  };
  let mut run = json!({ "kind": "http", "url": url, "format": format });
  if let Some(client) = provider.get("client") {
    run["client"] = client.clone();
  }
  let mut entry = json!({ "name": STATE_CHANGE, "run": run });
  if let Some(ms) = args.request_timeout {
    entry["timeout-ms"] = json!(ms);
  }
  let Some(config) = hooks.as_object_mut() else {
    return Err(String::from("the verifier configuration is not an object").into());
  };
  let points = config.entry("hooks").or_insert_with(|| json!({}));
  let Some(points) = points.as_object_mut() else {
    return Err(String::from("the verifier configuration's hooks are not an object").into());
  };
  points.insert("state-setup".to_string(), json!([entry.clone()]));
  if args.state_change_teardown {
    let teardown = points.entry("state-teardown").or_insert_with(|| json!([]));
    if let Some(entries) = teardown.as_array_mut() {
      entries.push(entry);
    }
  }
  Ok(())
}

/// Where the pacts come from: files and directories, a Pact Broker's selection, or pacts by URL,
/// with today's spellings for each (staged plan S1.13). One kind per run. A broker URL that only
/// the environment gave (`PACT_BROKER_BASE_URL`, which a CI job sets for every pact command) gives
/// way to files or URLs named on the command line. Beside it, the file each inline document was read
/// from, in order, so an error about one can name it.
fn source(args: &VerifyArgs) -> Result<(Value, Vec<String>), Error> {
  let conflict = |message: &str| error::usage(&["verify"], ErrorKind::ArgumentConflict, message);
  let mut files = args.sources.clone();
  files.extend(args.files.iter().cloned());
  files.extend(args.dirs.iter().cloned());
  let mut urls = args.urls.clone();
  let webhook = args.webhook_callback_url.is_some();
  if let Some(url) = &args.webhook_callback_url {
    if !urls.is_empty() || !files.is_empty() {
      return Err(conflict(
        "--webhook-callback-url verifies the one pact the webhook named; not with files or --url",
      ));
    }
    urls.push(url.clone());
  }
  let from_environment = std::env::var("PACT_BROKER_BASE_URL").ok();
  let broker_url = args
    .broker_url
    .as_ref()
    .filter(|url| !(files.len() + urls.len() > 0 && from_environment.as_ref() == Some(*url)));
  if !urls.is_empty() {
    // The webhook's broker is the one its pact came from; today's verifier needs both named.
    if broker_url.is_some() && !webhook {
      return Err(conflict("--url and --broker-url: one kind of source per run"));
    }
    if !files.is_empty() {
      return Err(conflict("--url and files: one kind of source per run"));
    }
    let mut source = json!({ "kind": "urls", "urls": urls });
    if let Some(auth) = broker_auth(args) {
      source["auth"] = auth;
    }
    if let Some(client) = broker_client(args) {
      source["client"] = client;
    }
    return Ok((source, Vec::new()));
  }
  if !files.is_empty() {
    if broker_url.is_some() {
      return Err(conflict("files and --broker-url: one kind of source per run"));
    }
    if args.publish {
      return Err(conflict(
        "--publish needs --broker-url or --url: a file has no broker to publish to",
      ));
    }
    let (paths, documents): (Vec<_>, Vec<_>) = io::read_all_documents(&files)?.into_iter().unzip();
    return Ok((json!({ "kind": "inline", "contracts": documents }), paths));
  }
  match broker_url {
    Some(url) => Ok((broker_source(args, url)?, Vec::new())),
    None => Err(error::usage(
      &["verify"],
      ErrorKind::MissingRequiredArgument,
      "say what to verify: contract or pact files, --broker-url with --provider-name, or --url",
    )),
  }
}

/// The broker's credentials, from today's options and variables: a token, else a username.
fn broker_auth(args: &VerifyArgs) -> Option<Value> {
  if let Some(token) = &args.token {
    return Some(json!({ "kind": "bearer", "token": token }));
  }
  args.user.as_ref().map(|user| {
    json!({ "kind": "basic", "username": user, "password": args.password.clone().unwrap_or_default() })
  })
}

/// The broker's HTTP client options: today's `--retries`, when given.
fn broker_client(args: &VerifyArgs) -> Option<Value> {
  args.retries.map(|retries| json!({ "retries": retries }))
}

/// A `broker` source (protocol spec §8.3) from the command line. The engine reads no credential
/// from its environment (§8.7): the CLI does, by today's names, and passes them in the descriptor.
fn broker_source(args: &VerifyArgs, url: &str) -> Result<Value, Error> {
  let Some(provider) = &args.provider_name else {
    return Err(error::usage(
      &["verify"],
      ErrorKind::MissingRequiredArgument,
      "--broker-url needs --provider-name: the provider whose pacts to fetch",
    ));
  };
  let mut selectors = Vec::with_capacity(args.consumer_version_selectors.len());
  for selector in &args.consumer_version_selectors {
    let parsed: Value = serde_json::from_str(selector)
      .map_err(|err| format!("--consumer-version-selectors '{selector}' is not JSON: {err}"))?;
    if !parsed.is_object() {
      return Err(format!("--consumer-version-selectors '{selector}' is not a JSON object").into());
    }
    selectors.push(parsed);
  }
  // Today's --consumer-version-tags: the latest pact for each tag (pact_verifier_cli's
  // consumer_tags_to_selectors).
  for tag in &args.consumer_version_tags {
    selectors.push(json!({ "tag": tag, "latest": true }));
  }
  let mut broker = json!({ "url": url });
  if let Some(auth) = broker_auth(args) {
    broker["auth"] = auth;
  }
  if let Some(client) = broker_client(args) {
    broker["client"] = client;
  }
  // Today's verifier asks for pending and work-in-progress pacts only when it has selectors to
  // send: with none, it fetches the latest pact from each consumer from an older endpoint that
  // knows neither (corpora/verifier/broker/pending-without-selectors). The broker's own default for
  // no selectors is the same pacts, so asking without them is what makes the two agree.
  let selecting = !selectors.is_empty();
  if !selecting && (args.enable_pending || args.include_wip_pacts_since.is_some()) {
    tracing::warn!(
      "--enable-pending and --include-wip-pacts-since need --consumer-version-selectors; without them, neither is asked for"
    );
  }
  let mut source = json!({
    "kind": "broker",
    "broker": broker,
    "provider": provider,
    "consumer-version-selectors": selectors,
    "include-pending": args.enable_pending && selecting,
  });
  if let Some(branch) = &args.provider_branch {
    source["provider-branch"] = json!(branch);
  }
  if !args.provider_tags.is_empty() {
    source["provider-tags"] = json!(args.provider_tags);
  }
  if let Some(since) = args.include_wip_pacts_since.as_ref().filter(|_| selecting) {
    source["include-wip-since"] = json!(since);
  }
  if args.ignore_no_pacts_error {
    source["require-pacts"] = json!(false);
  }
  Ok(source)
}

/// What `janus verify` prints as a run's events arrive.
///
/// A hook that fails an exchange, or finds its state unreachable, is reported by the result that
/// follows it too, so its event is held and printed under that result's line, once, where a reader
/// looks for why the interaction failed. A hook that aborts the run is the summary's headline, and
/// is printed there.
#[derive(Default)]
struct Reporter {
  held_hook: Option<Value>,
}

impl Reporter {
  /// One event, rendered. The vocabulary is open (spec §9.6), so an unrecognised kind is *printed*
  /// rather than dropped: a host that silently ignored an event kind it did not know would hide
  /// exactly the information a newer engine added to tell it something.
  fn report(&mut self, event: &Value) {
    let payload = &event["payload"];
    match event["kind"].as_str().unwrap_or_default() {
      "verification/started" => {
        let providers = join(&payload["providers"]);
        let formats = join(&payload["formats"]);
        outln!(
          "Verifying {} in {} [{formats}] against {providers}",
          count(&payload["interactions"], "interaction", "interactions"),
          count(&payload["contracts"], "document", "documents"),
        );
        if payload["filtered"] == json!(true) {
          outln!("  (filtered run: not every recorded variant will be replayed)");
        }
        outln!();
      }
      "verification/interaction-result" => self.result(payload),
      "verification/fetched" => {
        let pacts = payload["pacts"].as_array().cloned().unwrap_or_default();
        outln!(
          "Fetched {} from {}",
          count(&json!(pacts.len()), "pact", "pacts"),
          payload["broker"].as_str().unwrap_or("the broker")
        );
        for pact in &pacts {
          let consumer = pact["consumer"].as_str().unwrap_or("?");
          let status = match (pact["wip"] == json!(true), pact["pending"] == json!(true)) {
            (true, _) => " (work in progress, pending)",
            (false, true) => " (pending)",
            _ => "",
          };
          outln!("  {consumer}{status}");
          for notice in pact["notices"].as_array().into_iter().flatten() {
            outln!("    {}", notice.as_str().unwrap_or_default());
          }
        }
        outln!();
      }
      "verification/published" => {
        let consumer = payload["consumer"].as_str().unwrap_or("?");
        let version = payload["provider-version"].as_str().unwrap_or("?");
        match payload["outcome"].as_str() {
          Some("published") => outln!("  published {consumer}'s result for provider version {version}"),
          Some("skipped") => match payload["reason"].as_str() {
            Some("not-from-a-broker") => {
              outln!("  not publishing {consumer}'s result: its pact was not fetched from a Pact Broker")
            }
            _ => {
              outln!("  not publishing {consumer}'s result: the run's filter left some of its variants out")
            }
          },
          _ => {
            outln!("  PUBLISH FAILED for {consumer}'s result");
            print_error(&payload["error"]);
          }
        }
      }
      "verification/warning" => {
        let code = payload["code"].as_str().unwrap_or("warning");
        match (payload["target"].as_str(), payload["message"].as_str()) {
          (Some(target), Some(message)) => outln!("  warn    {code} at {target}: {message}"),
          (None, Some(message)) => outln!("  warn    {code}: {message}"),
          _ => outln!("  warn    {code}"),
        }
      }
      // `--explain-failures` is this event arriving: the executed plan for the variant that just
      // failed, printed where the failure is, rather than left for a second command to re-derive.
      "verification/executed-plan" => {
        outln!();
        outln!(
          "  --- executed plan: {}{} ---",
          payload["interaction"]["description"].as_str().unwrap_or("?"),
          variant_label(payload)
        );
        for line in highlight::plan(payload["text"].as_str().unwrap_or("")).lines() {
          outln_styled!("  {line}");
        }
        outln!();
      }
      "verification/hook" => {
        let outcome = payload["outcome"].as_str().unwrap_or("ok");
        if outcome == "continue" || outcome == "ok" {
          return;
        }
        match payload["effect"].as_str() {
          Some("failed-exchange" | "state-unavailable") => self.held_hook = Some(payload.clone()),
          // Its error is the summary's headline, where the run's end is explained.
          Some("aborted-run") => outln!(
            "  hook    {} at {}: {}, and aborted the run",
            payload["hook"].as_str().unwrap_or("?"),
            payload["point"].as_str().unwrap_or("?"),
            outcome
          ),
          _ => {
            print_hook(payload, "  hook    ");
            print_error(&payload["error"]);
          }
        }
      }
      // The result line that follows says everything these two carry; printing them as well would
      // double every run's output for no reader's benefit.
      "verification/interaction-started" | "verification/finished" => {}
      other => outln!("  event   {other}: {payload}"),
    }
  }

  fn result(&mut self, payload: &Value) {
    let line = format!(
      "{}{}",
      payload["interaction"]["description"].as_str().unwrap_or("?"),
      variant_label(payload)
    );
    match payload["status"].as_str().unwrap_or("failed") {
      "verified" => outln!("  ok      {line}"),
      "state-unavailable" => {
        outln!("  STATE   {line}");
        outln!("          the provider cannot reach a state this contract declares");
      }
      _ if payload["pending"] == json!(true) => {
        outln!("  PENDING {line}");
        outln!("          failed, and pending: reported, and not failing the run");
      }
      _ => outln!("  FAILED  {line}"),
    }
    for mismatch in payload["mismatches"].as_array().into_iter().flatten() {
      outln!(
        "          {}: {}",
        mismatch["path"].as_str().unwrap_or(""),
        mismatch["message"].as_str().unwrap_or("")
      );
    }
    // The hook's error is the result's: said once, after which hook it was.
    if let Some(hook) = self.held_hook.take() {
      print_hook(&hook, "          ");
    }
    print_error(&payload["error"]);
  }
}

/// `hook auth at before-request: failed`, after `indent`.
fn print_hook(payload: &Value, indent: &str) {
  outln!(
    "{indent}{} at {}: {}",
    payload["hook"].as_str().unwrap_or("?"),
    payload["point"].as_str().unwrap_or("?"),
    payload["outcome"].as_str().unwrap_or("?")
  );
}

/// ` [shipped]`, the variant a result is about, or nothing for a v1–v4 pact's: one example is one
/// variant, always `base`, and naming it on every line says nothing.
fn variant_label(payload: &Value) -> String {
  let pact = payload["interaction"]["format"]
    .as_str()
    .is_some_and(|format| format.starts_with("pact/"));
  match payload["variant"].as_str() {
    Some(_) if pact => String::new(),
    Some(variant) => format!(" [{variant}]"),
    None => String::new(),
  }
}

/// `1 interaction`, `2 interactions`.
fn count(n: &Value, one: &str, many: &str) -> String {
  match n.as_u64() {
    Some(1) => format!("1 {one}"),
    Some(n) => format!("{n} {many}"),
    None => format!("? {many}"),
  }
}

/// An error under the result or hook line it belongs to: every line of it at the same indent.
fn print_error(error: &Value) {
  if error.is_null() {
    return;
  }
  const INDENT: &str = "          ";
  outln!(
    "{INDENT}{}",
    engine::render_error(error).replace('\n', &format!("\n{INDENT}"))
  );
}

/// `FAILED: 0 verified, 2 failed (2 interactions)`: what the run reached, the tallies that are not
/// zero beside the two that always matter, and how much there was. Variants are named only when
/// there are more of them than interactions, as there are only in a contract.
fn render_summary(summary: &Value) -> String {
  let status = summary["status"].as_str().unwrap_or("failed").to_uppercase();
  let variants = &summary["variants"];
  let tally = |name: &str| variants[name].as_u64().unwrap_or(0);
  let mut counts = vec![
    format!("{} verified", tally("verified")),
    format!("{} failed", tally("failed")),
  ];
  for (name, label) in [
    ("state-unavailable", "state-unavailable"),
    ("skipped", "skipped"),
    // `variants.total` counts what the run reached; an aborted or stopped run reached less than it
    // planned, and the rest is counted beside the tallies, never folded into them.
    ("not-run", "not run"),
  ] {
    if tally(name) > 0 {
      counts.push(format!("{} {label}", tally(name)));
    }
  }
  let total = tally("total") + tally("not-run");
  let interactions = summary["interactions"].as_u64().unwrap_or(0);
  let scope = if total == interactions {
    count(&json!(interactions), "interaction", "interactions")
  } else {
    format!(
      "{} across {}",
      count(&json!(total), "variant", "variants"),
      count(&json!(interactions), "interaction", "interactions")
    )
  };
  let mut text = format!("{status}: {} ({scope})", counts.join(", "));
  if summary["filtered"] == json!(true) {
    text.push_str("\nThis was a filtered run: unreplayed variants are not passing ones.");
  }
  if summary["stopped"] == json!("first-failure") {
    text.push_str("\nThe run stopped at its first failure, as --exit-on-first-error asked: what it did not reach was not run.");
  }
  // The abort is the headline of a run that ended this way, so it carries its cause: "the auth
  // hook failed" sends a reader to the logs for what the report already knows.
  if let Some(abort) = summary.get("aborted").filter(|a| !a.is_null()) {
    text.push_str(&format!(
      "\nThe run was aborted by the '{}' hook at {}",
      abort["hook"].as_str().unwrap_or("?"),
      abort["point"].as_str().unwrap_or("?")
    ));
    match abort.get("error").filter(|e| !e.is_null()) {
      Some(error) => text.push_str(&format!(
        ":\n  {}",
        engine::render_error(error).replace('\n', "\n  ")
      )),
      None => text.push('.'),
    }
  }
  text
}

fn join(value: &Value) -> String {
  value
    .as_array()
    .map(|items| {
      items
        .iter()
        .map(|item| item.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>()
        .join(", ")
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::{render_summary, variant_label};
  use pretty_assertions::assert_eq;
  use serde_json::json;

  /// The tallies that are zero are left out, but for the two that always matter; variants are named
  /// only when there are more of them than interactions.
  #[test]
  fn a_summary_says_what_the_run_reached_and_how_much_there_was() {
    let pact_run = json!({
      "status": "failed", "interactions": 2,
      "variants": { "total": 2, "verified": 0, "failed": 2, "state-unavailable": 0, "skipped": 0 },
    });
    assert_eq!(
      render_summary(&pact_run),
      "FAILED: 0 verified, 2 failed (2 interactions)"
    );

    let contract_run = json!({
      "status": "failed", "interactions": 1, "stopped": "first-failure",
      "variants": { "total": 2, "verified": 1, "failed": 1, "state-unavailable": 1, "skipped": 0, "not-run": 3 },
    });
    assert_eq!(
      render_summary(&contract_run),
      "FAILED: 1 verified, 1 failed, 1 state-unavailable, 3 not run (5 variants across 1 interaction)\n\
       The run stopped at its first failure, as --exit-on-first-error asked: what it did not reach was not run."
    );
  }

  #[test]
  fn a_pacts_one_variant_goes_unnamed() {
    let pact = json!({ "interaction": { "format": "pact/3.0.0" }, "variant": "base" });
    let contract = json!({ "interaction": { "format": "janus-contract/1" }, "variant": "shipped" });
    assert_eq!(variant_label(&pact), "");
    assert_eq!(variant_label(&contract), " [shipped]");
  }
}

/// Today's options, as the engine receives them (staged plan S1.13, ADR 0053): each command line is
/// parsed as clap parses it and turned into the request's source, options and hooks.
#[cfg(test)]
mod todays_options {
  use super::*;
  use crate::args::{Cli, Command};
  use clap::Parser;
  use pretty_assertions::assert_eq;

  fn verify_args(line: &[&str]) -> VerifyArgs {
    let mut argv = vec!["janus", "verify"];
    argv.extend(line);
    match Cli::try_parse_from(argv)
      .expect("the command line parses")
      .command
    {
      Command::Verify(args) => args,
      other => panic!("not verify: {other:?}"),
    }
  }

  #[test]
  fn consumer_version_tags_are_the_latest_pact_for_each_tag_and_retries_the_brokers_client() {
    let args = verify_args(&[
      "-b",
      "https://broker.example",
      "-n",
      "orders",
      "--provider-url",
      "http://localhost:8080",
      "--consumer-version-tags",
      "main,prod",
      "--retries",
      "8",
      "--ignore-no-pacts-error",
      "--token",
      "t",
    ]);
    let (source, _) = source(&args).unwrap();
    assert_eq!(
      source["consumer-version-selectors"],
      json!([{ "tag": "main", "latest": true }, { "tag": "prod", "latest": true }])
    );
    assert_eq!(source["broker"]["client"], json!({ "retries": 8 }));
    assert_eq!(
      source["broker"]["auth"],
      json!({ "kind": "bearer", "token": "t" })
    );
    assert_eq!(source["require-pacts"], json!(false));
  }

  #[test]
  fn a_url_and_a_webhooks_url_are_pacts_fetched_with_the_brokers_credentials() {
    let args = verify_args(&[
      "--url",
      "https://broker.example/pacts/1",
      "--url",
      "https://files.example/p.json",
      "--provider-url",
      "http://localhost:8080",
      "--user",
      "bob",
      "--password",
      "pw",
    ]);
    assert_eq!(
      source(&args).unwrap().0,
      json!({
        "kind": "urls", "urls": ["https://broker.example/pacts/1", "https://files.example/p.json"],
        "auth": { "kind": "basic", "username": "bob", "password": "pw" },
      })
    );
    // Today's verifier needs the webhook's broker named too; it is the one the pact came from.
    let webhook = verify_args(&[
      "--webhook-callback-url",
      "https://broker.example/pacts/2",
      "--broker-url",
      "https://broker.example",
      "--provider-url",
      "http://localhost:8080",
    ]);
    assert_eq!(
      source(&webhook).unwrap().0,
      json!({ "kind": "urls", "urls": ["https://broker.example/pacts/2"] })
    );
  }

  #[rstest::rstest]
  #[case::url_and_broker(&["--url", "https://a/p", "-b", "https://b", "-n", "p"])]
  #[case::url_and_file(&["--url", "https://a/p", "-f", "pact.json"])]
  #[case::webhook_and_file(&["--webhook-callback-url", "https://a/p", "pact.json"])]
  #[case::nothing(&[])]
  #[case::publish_from_files(&["pact.json", "--publish", "--provider-version", "1"])]
  fn one_kind_of_source_per_run(#[case] line: &[&str]) {
    let mut line = line.to_vec();
    line.extend(["--provider-url", "http://localhost:8080"]);
    assert!(matches!(source(&verify_args(&line)), Err(Error::Usage(_))));
  }

  #[test]
  fn hostname_port_and_https_are_the_provider_url_and_the_http_transport() {
    let args = verify_args(&[
      "pact.json",
      "--hostname",
      "orders.local",
      "--port",
      "8443",
      "--transport",
      "https",
    ]);
    assert_eq!(provider_url(&args).unwrap(), "https://orders.local:8443");
    assert_eq!(transport_kind(&args), "http");
    let defaulted = verify_args(&["pact.json", "-p", "8080"]);
    assert_eq!(provider_url(&defaulted).unwrap(), "http://localhost:8080");
    assert!(matches!(
      provider_url(&verify_args(&["pact.json"])),
      Err(Error::Usage(_))
    ));
  }

  #[test]
  fn todays_state_change_options_are_the_hooks_they_become() {
    let args = verify_args(&[
      "pact.json",
      "--provider-url",
      "http://localhost:8080",
      "-s",
      "http://localhost:8080/states",
      "--state-change-as-query",
      "--state-change-teardown",
      "--request-timeout",
      "2000",
    ]);
    let mut hooks = json!({ "version": 1 });
    add_state_change(&mut hooks, &args, &json!({})).unwrap();
    let entry = json!({
      "name": "state-change-url", "timeout-ms": 2000,
      "run": { "kind": "http", "url": "http://localhost:8080/states", "format": "pact-state-change-query" },
    });
    assert_eq!(hooks["hooks"]["state-setup"], json!([entry]));
    assert_eq!(hooks["hooks"]["state-teardown"], json!([entry]));

    let mut configured = json!({ "version": 1, "hooks": { "state-setup": [{ "name": "mine" }] } });
    assert!(matches!(
      add_state_change(&mut configured, &args, &json!({})),
      Err(Error::Usage(_))
    ));
  }

  #[test]
  fn the_ruby_verifiers_spellings_are_read_as_janus_verifys() {
    let args = verify_args(&[
      "--provider-base-url",
      "http://localhost:8080",
      "--pact-broker-base-url",
      "https://broker.example",
      "--provider",
      "orders",
      "--provider-app-version",
      "1.2.3",
      "--provider-version-branch",
      "main",
      "--publish-verification-results",
      "--broker-username",
      "bob",
      "--custom-provider-header",
      "Authorization: Bearer x",
      "--provider-states-setup-url",
      "http://localhost:8080/states",
    ]);
    assert_eq!(args.provider_url.as_deref(), Some("http://localhost:8080"));
    assert_eq!(args.broker_url.as_deref(), Some("https://broker.example"));
    assert_eq!(args.provider_name.as_deref(), Some("orders"));
    assert_eq!(args.provider_version.as_deref(), Some("1.2.3"));
    assert_eq!(args.provider_branch.as_deref(), Some("main"));
    assert!(args.publish);
    assert_eq!(args.user.as_deref(), Some("bob"));
    assert_eq!(
      args.custom_provider_header,
      vec![("Authorization".to_string(), "Bearer x".to_string())]
    );
    assert_eq!(
      args.state_change_url.as_deref(),
      Some("http://localhost:8080/states")
    );
  }
}
