---
default: minor
---

# A run that ended early counts what it did not reach, and errors say why in their message

The second part of staged plan S1.29's review of the CLI's output.

- **`variants.not-run`.** The summary of a run that `stop-on-first-failure` stopped, or that a hook
  aborted, now counts in `variants.not-run` the recorded variants it did not reach. No other tally
  counts them, and `total` is unchanged. Until now, a stopped run did not say how much it left, and an
  aborted run said it only in the hook report, which a run with no hooks does not have.
- **Messages carry what a person needs, so a host can show the message alone:**
  - `variant-not-found` from `verification/verify` names the ids the documents do record ("…; they
    record 'base'").
  - An `http` hook whose endpoint answers with an error status quotes up to 200 characters of the body
    in its `hook-http-failed` message. The details still carry the whole excerpt.
  - The excerpt no longer drops the body's last character.
- **Shorter error chains:**
  - The engine's HTTP client leaves out the layers that only name a library ("error sending request",
    "client error (Connect)", "tcp connect error") and the platform's `(os error N)`, so a refused
    connection reads `could not connect: Connection refused`.
  - `broker-unreachable`'s message is the request and its cause, without "the Pact Broker could not be
    reached", which was wrong for a pact fetched by its URL.
  - The OCI loader and the oauth2 component say "could not reach the registry" and "could not connect"
    instead of ureq's `io:`.
  - A registry that wants credentials nobody declared says so in words.

`janus verify` follows:
- **Exit status:** a run the engine ends before it verifies anything exits 2, with the reason on
  standard error. That covers a broker or URL it could not fetch from, no pacts selected, and a variant
  nothing records. It used to exit 1, as `pact_verifier_cli` does.
- **Errors:**
  - An error's `details` are no longer printed as raw JSON.
  - A component's failure leads with the component's own code (`transport-failed: GET …`).
  - A hook that fails an exchange is printed once, under the result it failed.
- **Results and summary:**
  - A v1–v4 pact's results no longer end in `[base]`.
  - The summary leaves out tallies that are zero, apart from verified and failed, uses plurals, and
    counts what a stopped run did not reach: `FAILED: 0 verified, 1 failed, 1 not run (2 interactions)`.
- **Smaller fixes:**
  - File errors drop `(os error N)`.
  - A file that is not JSON says so.
  - `component pull` puts its code first.
  - `explain --index first` says it wants a whole number.
