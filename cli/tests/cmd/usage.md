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
  component    Publish a WASM component to an OCI registry, or fetch one and say what it is
  completions  Print a shell completion script
  version      Print the CLI, engine and protocol versions
  help         Print this message or the help of the given subcommand(s)

Options:
      --log-level <LEVEL>  How much to log on standard error. Without it, RUST_LOG is read; the
                           default is warn [possible values: error, warn, info, debug, trace]
  -h, --help               Print help
  -V, --version            Print version

Every command drives the same engine an SDK embeds. --log-level debug shows the protocol frames it
exchanges.

```

```console
$ janus frobnicate
? 2
error: unrecognized subcommand 'frobnicate'

Usage: janus [OPTIONS] <COMMAND>

For more information, try '--help'.

```

```console
$ janus verify pact.json --nonsense
? 2
error: unexpected argument '--nonsense' found

  tip: to pass '--nonsense' as a value, use '-- --nonsense'

Usage: janus verify --provider-url <URL> <CONTRACT_OR_PACT>...

For more information, try '--help'.

```

```console
$ janus verify pact.json
? 2
error: the following required arguments were not provided:
  --provider-url <URL>

Usage: janus verify --provider-url <URL> <CONTRACT_OR_PACT>...

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
$ janus explain pact.json --index first
? 2
error: invalid value 'first' for '--index <N>': invalid digit found in string

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
error: the following required arguments were not provided:
  --provider-name <NAME>

Usage: janus verify --provider-url <URL> --broker-url <URL> --provider-name <NAME> [CONTRACT_OR_PACT]...

For more information, try '--help'.

```

```console
$ janus verify pact.json --provider-url http://127.0.0.1:1 --broker-url http://127.0.0.1:2 --provider-name p
? 2
error: the argument '[CONTRACT_OR_PACT]...' cannot be used with '--broker-url <URL>'

Usage: janus verify --provider-url <URL> --provider-name <NAME> <CONTRACT_OR_PACT>...

For more information, try '--help'.

```

Publishing needs a broker to publish to, and the provider version the results are for.

```console
$ janus verify --provider-url http://127.0.0.1:1 --broker-url http://127.0.0.1:2 --provider-name p --publish
? 2
error: the following required arguments were not provided:
  --provider-version <VERSION>

Usage: janus verify --provider-url <URL> --broker-url <URL> --provider-name <NAME> --provider-version <VERSION> --publish [CONTRACT_OR_PACT]...

For more information, try '--help'.

```
