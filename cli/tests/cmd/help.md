# The `janus` command line

Every command's `--help`, as the binaries print it. This file is a test (`tests/cmd.rs`): when a
flag changes, so does this page, and the diff is the review.

```console
$ janus --help
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
      --log-level <LEVEL>  How much to log on standard error. Without it, RUST_LOG is read; the
                           default is warn [possible values: error, warn, info, debug, trace]
  -h, --help               Print help
  -V, --version            Print version

Every command drives the same engine an SDK embeds. --log-level debug shows the protocol frames it
exchanges.

```

```console
$ janus verify --help
Replay Janus contracts or v1-v4 pacts at a running provider

Usage: janus verify [OPTIONS] --provider-url <URL> [CONTRACT_OR_PACT]...

Arguments:
  [CONTRACT_OR_PACT]...  Janus contracts or v1-v4 pact files, or directories of them. Not with
                         --broker-url

Options:
      --log-level <LEVEL>   How much to log on standard error. Without it, RUST_LOG is read; the
                            default is warn [possible values: error, warn, info, debug, trace]
      --provider-url <URL>  The base URL of the provider under test
      --config <FILE>       A verifier configuration (verifier.janus.yaml): the hooks that set up
                            provider states or authenticate, and the components the contracts need
      --variant <ID>        Replay only this variant; repeatable. The run is reported as filtered,
                            because a partial run is not a pass
      --transport <KIND>    The transport to bind to the provider [default: http]
      --explain-failures    Print the executed plan for every variant that fails
      --output <FORMAT>     text prints progress and a summary; json prints only the run summary
                            document [default: text] [possible values: text, json]
  -h, --help                Print help

Pact Broker:
      --broker-url <URL>
          Fetch the pacts to verify from this Pact Broker instead of from files
      --provider-name <NAME>
          The provider whose pacts to fetch
      --consumer-version-selectors <JSON>
          A consumer version selector, as the broker reads it, in JSON; repeatable
          (https://docs.pact.io/pact_broker/advanced_topics/consumer_version_selectors)
      --enable-pending
          Ask the broker which pacts are pending: their failures are reported, and do not fail the
          run. Only with --consumer-version-selectors, as today's verifier
      --include-wip-pacts-since <DATE>
          Also verify the work-in-progress pacts published since this date (YYYY-MM-DD); they are
          pending. Only with --consumer-version-selectors, as today's verifier
      --provider-version <VERSION>
          The provider version being verified, which --publish publishes results for
      --provider-branch <BRANCH>
          The provider version's branch, which the broker decides pending status from
      --provider-tags <TAGS>
          The provider version's tags, comma-separated
      --publish
          Publish each pact's result back to the broker
      --build-url <URL>
          The build that ran this verification, recorded with each published result
      --user <USER>
          The broker's username, for basic authentication [env: PACT_BROKER_USERNAME]
      --password <PASSWORD>
          The broker's password, for basic authentication [env: PACT_BROKER_PASSWORD]
      --token <TOKEN>
          A bearer token for the broker; used instead of --user when both are given [env:
          PACT_BROKER_TOKEN]

Exit status: 0 verified; 1 the provider failed verification, or the run was aborted; 2 the command
could not run.

```

```console
$ janus check --help
Decide whether a consumer can deploy against a provider, from documents that already exist

Usage: janus check [OPTIONS] <CONTRACT_OR_PACT>...

Arguments:
  <CONTRACT_OR_PACT>...  Consumer contracts or v1-v4 pacts, or directories of them

Options:
      --log-level <LEVEL>      How much to log on standard error. Without it, RUST_LOG is read; the
                               default is warn [possible values: error, warn, info, debug, trace]
      --provider-shape <PATH>  A provider shape document, or a directory of them; repeatable
      --verification <FILE>    A run summary written by `janus verify --output json`; repeatable
      --policy <FILE>          A subsumption policy (YAML or JSON): when to warn and when to block,
                               and any exemptions
      --on-finding <SEVERITY>  Override the policy for findings the check decided: warn reports
                               them, block also exits 1 [possible values: warn, block]
      --on-review <SEVERITY>   Override the policy for comparisons the check could not decide: warn
                               or block, as above [possible values: warn, block]
      --as-of <YYYY-MM-DD>     Judge exemption expiry against this date instead of today (UTC)
      --output <FORMAT>        text prints the report as prose; json prints the compatibility report
                               document [default: text] [possible values: text, json]
  -h, --help                   Print help

Exit status: 0 the consumer can deploy, possibly with warnings; 1 something blocked; 2 the command
could not run. Both severities default to warn.

This command reads documents and decides; it never contacts the provider. Verify first (janus verify
... --output json > result.json) and pass the result with --verification: a pair with no
verification result is reported as unverified, which is not a pass.

```

```console
$ janus explain --help
Print the plan the engine will execute for one interaction

Usage: janus explain [OPTIONS] <DOCUMENT>

Arguments:
  <DOCUMENT>  An interaction specification, a Janus contract, or a v1-v4 pact

Options:
      --index <N>          Which interaction of a contract or pact, counting from 0
      --log-level <LEVEL>  How much to log on standard error. Without it, RUST_LOG is read; the
                           default is warn [possible values: error, warn, info, debug, trace]
      --variant <ID>       Compile for this recorded variant of a Janus contract
      --spec               Read the document as an interaction specification, which cannot identify
                           itself
      --plan               Print the structured plan document (JSON) instead of the text form
      --executed <VALUES>  Run the plan against a JSON object of captured values (path to value) and
                           print the executed form. Exits 1 if the values do not match
  -h, --help               Print help

To see why a live verification failed, use janus verify --explain-failures instead: it prints the
executed plan against what the provider actually sent.

```

```console
$ janus upgrade --help
Convert a v1-v4 pact into a Janus contract, and say what the conversion could not carry

Usage: janus upgrade [OPTIONS] <PACT>

Arguments:
  <PACT>  The v1-v4 pact to convert

Options:
      --log-level <LEVEL>  How much to log on standard error. Without it, RUST_LOG is read; the
                           default is warn [possible values: error, warn, info, debug, trace]
      --write <FILE>       Write the contract to this file instead of standard output
      --quiet              Do not print the findings
      --output <FORMAT>    text prints the contract, and the findings on standard error; json prints
                           one document holding both [default: text] [possible values: text, json]
  -h, --help               Print help

A converted pact has one variant per interaction, because one example is one variant; growing it is
the consumer test suite's job. Verifying a pact needs no conversion: janus verify reads v1-v4 pacts
directly.

```

```console
$ janus publish --help
Publish Janus contracts or v1-v4 pacts to a Pact Broker, for a version of their consumer

Usage: janus publish [OPTIONS] --consumer-app-version <VERSION> --broker-base-url <URL> <CONTRACT_OR_PACT>...

Arguments:
  <CONTRACT_OR_PACT>...  Janus contracts or v1-v4 pact files, or directories of them

Options:
  -a, --consumer-app-version <VERSION>  The version of the consumer these contracts belong to
      --log-level <LEVEL>               How much to log on standard error. Without it, RUST_LOG is
                                        read; the default is warn [possible values: error, warn,
                                        info, debug, trace]
      --branch <BRANCH>                 The consumer version's branch
  -t, --tag <TAG>                       A tag for the consumer version; repeatable, or
                                        comma-separated
      --build-url <URL>                 The build that produced the contracts
      --output <FORMAT>                 text prints what was published and the broker's notices;
                                        json prints one document [default: text] [possible values:
                                        text, json]
  -h, --help                            Print help

Pact Broker:
  -b, --broker-base-url <URL>       The Pact Broker's base URL [env: PACT_BROKER_BASE_URL]
  -u, --broker-username <USER>      The broker's username, for basic authentication [env:
                                    PACT_BROKER_USERNAME]
  -p, --broker-password <PASSWORD>  The broker's password, for basic authentication [env:
                                    PACT_BROKER_PASSWORD]
  -k, --broker-token <TOKEN>        A bearer token for the broker; used instead of --broker-username
                                    when both are given [env: PACT_BROKER_TOKEN]

Every contract is published as the broker's 'pact' specification, which every Pact Broker accepts.
Contracts from several consumers are published one consumer at a time, each for the same version.

Exit status: 0 published; 2 something was not published (the broker refused it or could not be
reached), or the command could not run.

```

```console
$ janus component --help
Publish a WASM component to an OCI registry, or fetch one and say what it is

Usage: janus component [OPTIONS] <COMMAND>

Commands:
  push  Publish a WASM component as an OCI artifact, described by the component's own handshake
  pull  Fetch a component into the cache, check every byte, and print how to declare it, pinned
  help  Print this message or the help of the given subcommand(s)

Options:
      --log-level <LEVEL>  How much to log on standard error. Without it, RUST_LOG is read; the
                           default is warn [possible values: error, warn, info, debug, trace]
  -h, --help               Print help

Loopback registries (localhost, 127.0.0.1) are plain HTTP; everything else is HTTPS. Credentials,
when a registry wants them: JANUS_OCI_USERNAME and JANUS_OCI_PASSWORD. The cache is
JANUS_COMPONENT_CACHE, or pact-janus/components in the user cache directory.

Exit status: 0 done; 1 the artifact is not what it claims (a digest that does not match, not a Janus
component, a config that disagrees with the component); 2 the command could not run.

```

```console
$ janus component push --help
Publish a WASM component as an OCI artifact, described by the component's own handshake

Usage: janus component push [OPTIONS] <COMPONENT_WASM> <REFERENCE>

Arguments:
  <COMPONENT_WASM>  The component to publish
  <REFERENCE>       Where to publish it: registry/repository:tag

Options:
      --log-level <LEVEL>  How much to log on standard error. Without it, RUST_LOG is read; the
                           default is warn [possible values: error, warn, info, debug, trace]
      --output <FORMAT>    text prints a summary and the declaration to paste; json prints one
                           document [default: text] [possible values: text, json]
  -h, --help               Print help

```

```console
$ janus component pull --help
Fetch a component into the cache, check every byte, and print how to declare it, pinned

Usage: janus component pull [OPTIONS] <REFERENCE>

Arguments:
  <REFERENCE>  What to fetch: registry/repository:tag, or registry/repository@sha256:...

Options:
      --digest <sha256:...>  The manifest digest the artifact must match
      --log-level <LEVEL>    How much to log on standard error. Without it, RUST_LOG is read; the
                             default is warn [possible values: error, warn, info, debug, trace]
      --output <FORMAT>      text prints a summary and the declaration to paste; json prints one
                             document [default: text] [possible values: text, json]
  -h, --help                 Print help

```

```console
$ janus completions --help
Print a shell completion script

Usage: janus completions [OPTIONS] <SHELL>

Arguments:
  <SHELL>  The shell to generate the script for [possible values: bash, elvish, fish, powershell,
           zsh]

Options:
      --log-level <LEVEL>  How much to log on standard error. Without it, RUST_LOG is read; the
                           default is warn [possible values: error, warn, info, debug, trace]
  -h, --help               Print help

For example, for bash: janus completions bash > ~/.local/share/bash-completion/completions/janus

```

```console
$ janus-engine --help
The Janus engine as a subprocess: protocol frames on standard input and output, logs on standard
error. SDKs start it; RUST_LOG sets what it logs (default: info).

Usage: janus-engine

Options:
  -h, --help     Print help
  -V, --version  Print version

```
