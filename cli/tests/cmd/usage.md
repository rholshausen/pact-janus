# Usage errors

Each exits 2: the command could not run. This file is a test (`tests/cmd.rs`).

A command line that cannot be parsed exits 2, with the usage of the command it is about.

```console
$ janus
? 2
Verify contracts and pacts against a provider, explain matching plans, upgrade pacts, and decide
whether a consumer can deploy

Usage: janus [OPTIONS] <COMMAND>

Commands:
  verify       Replay Janus contracts or v1-v4 pacts at a running provider
  check        Decide whether a consumer can deploy against a provider, from documents that already
               exist
  explain      Print the plan the engine will execute for one interaction
  upgrade      Convert a v1-v4 pact into a Janus contract, and say what the conversion could not
               carry
  publish      Publish Janus contracts or v1-v4 pacts to a Pact Broker, for a version of their
               consumer
  component    Publish a WASM component to an OCI registry, or fetch one and say what it is
  completions  Print a shell completion script
  version      Print the CLI, engine and protocol versions
  help         Print this message or the help of the given subcommand(s)

Options:
  -l, --log-level <LEVEL>  How much to log on standard error. Without it, RUST_LOG is read; the
                           default is warn [possible values: off, error, warn, info, debug, trace]
      --no-colour          Print no colour, on a terminal too (as NO_COLOR does)
  -h, --help               Print help
  -V, --version            Print version

Every command drives the same engine an SDK embeds. --log-level trace shows the protocol frames it
exchanges.

```

```console
$ janus frobnicate
? 2
error: unrecognized subcommand 'frobnicate'

Usage: janus [OPTIONS] <COMMAND>

For more information, try '--help'.

```

A command given none of its arguments prints its help, the same text as `--help` (`help.md`
pins it whole).

```console
$ janus verify
? 2
Replay Janus contracts or v1-v4 pacts at a running provider
...
```

```console
$ janus check
? 2
Decide whether a consumer can deploy against a provider, from documents that already exist
...
```

```console
$ janus explain
? 2
Print the plan the engine will execute for one interaction
...
```

```console
$ janus upgrade
? 2
Convert a v1-v4 pact into a Janus contract, and say what the conversion could not carry
...
```

```console
$ janus publish
? 2
Publish Janus contracts or v1-v4 pacts to a Pact Broker, for a version of their consumer
...
```

```console
$ janus component push
? 2
Publish a WASM component as an OCI artifact, described by the component's own handshake
...
```

```console
$ janus component pull
? 2
Fetch a component into the cache, check every byte, and print how to declare it, pinned
...
```

```console
$ janus completions
? 2
Print a shell completion script
...
```

```console
$ janus verify pact.json --nonsense
? 2
error: unexpected argument '--nonsense' found

  tip: to pass '--nonsense' as a value, use '-- --nonsense'

Usage: janus verify <CONTRACT_OR_PACT>...

For more information, try '--help'.

```

```console
$ janus verify pact.json
? 2
error: say where the provider is: --provider-url <URL>, or --hostname and --port

Usage: janus verify [OPTIONS] [CONTRACT_OR_PACT]...

For more information, try '--help'.

```

Values are checked before anything runs.

```console
$ janus check pact.json --on-finding maybe
? 2
error: invalid value 'maybe' for '--on-finding <SEVERITY>'
  [possible values: warn, block]

For more information, try '--help'.

```

```console
$ janus check pact.json --as-of 2026-02-30
? 2
error: invalid value '2026-02-30' for '--as-of <YYYY-MM-DD>': 2026-02-30 is not a day in the calendar

For more information, try '--help'.

```

```console
$ janus verify pact.json --provider-url http://127.0.0.1:1 --transports message
? 2
error: invalid value 'message' for '--transports <NAME:PORT[/PATH]>': 'message' is not a transport: it must be NAME:PORT[/PATH]

For more information, try '--help'.

```

```console
$ janus explain pact.json --index first
? 2
error: invalid value 'first' for '--index <N>': not a position: it must be a whole number, counting from 0

For more information, try '--help'.

```

```console
$ janus verify pact.json --provider-url localhost
? 2
error: invalid value 'localhost' for '--provider-url <URL>': not a URL: relative URL without a base

For more information, try '--help'.

```

Flags that cannot be used together.

```console
$ janus explain pact.json --spec --index 1
? 2
error: the argument '--spec' cannot be used with '--index <N>'

Usage: janus explain --spec <DOCUMENT>

For more information, try '--help'.

```

```console
$ janus explain pact.json --plan --executed values.json
? 2
error: the argument '--plan' cannot be used with '--executed <VALUES>'

Usage: janus explain --plan <DOCUMENT>

For more information, try '--help'.

```

```console
$ janus upgrade pact.json --output json --write contract.json
? 2
error: the argument '--output json' cannot be used with '--write <FILE>'

Usage: janus upgrade [OPTIONS] <PACT>

For more information, try '--help'.

```

A file that cannot be read is the command failing to run: exit 2, named.

```console
$ janus upgrade no-such-pact.json
? 2
janus upgrade: no-such-pact.json: [..]

```

Verifying from a Pact Broker needs the provider whose pacts to fetch, and files and a broker are
not mixed in one run.

```console
$ janus verify --provider-url http://127.0.0.1:1 --broker-url http://127.0.0.1:2
? 2
error: --broker-url needs --provider-name: the provider whose pacts to fetch

Usage: janus verify [OPTIONS] [CONTRACT_OR_PACT]...

For more information, try '--help'.

```

```console
$ janus verify pact.json --provider-url http://127.0.0.1:1 --broker-url http://127.0.0.1:2 --provider-name p
? 2
error: files and --broker-url: one kind of source per run

Usage: janus verify [OPTIONS] [CONTRACT_OR_PACT]...

For more information, try '--help'.

```

Publishing needs a broker to publish to, and the provider version the results are for.

```console
$ janus verify --provider-url http://127.0.0.1:1 --broker-url http://127.0.0.1:2 --provider-name p --publish
? 2
error: the following required arguments were not provided:
  --provider-version <VERSION>

Usage: janus verify --provider-version <VERSION> --provider-url <URL> --broker-url <URL> --provider-name <NAME> --publish [CONTRACT_OR_PACT]...

For more information, try '--help'.

```

Publishing contracts needs the consumer version they belong to.

```console
$ janus publish pact.json --broker-base-url http://127.0.0.1:2
? 2
error: the following required arguments were not provided:
  --consumer-app-version <VERSION>

Usage: janus publish --consumer-app-version <VERSION> --broker-base-url <URL> <CONTRACT_OR_PACT>...

For more information, try '--help'.

```
