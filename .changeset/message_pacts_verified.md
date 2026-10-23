---
default: major
---

# Message pacts are verified

- **Breaking: a v1–v4 message interaction is verified** where it was skipped. A v3 message, and a v4
  asynchronous or synchronous message, is asked of the provider by the run's `produce-message` hooks,
  which produce a `message` part (`metadata` and `body` slots), and compared with the message the
  pact expects as today's verifier compares one (ADR 0046). A message pact whose run used to report
  `verified` with nothing verified now fails unless the provider produces its messages.
- **When no hook produces a message**, the variant fails as `component-unavailable` with
  `details.component` `hook/produce-message` and `details.reason` `message-not-produced`.
- **The `http` hook has `format: pact-message`**: today's request for a message (the hook context's
  new `interaction.pact-message.request`), and the reply read as the message. Its `transports` member
  routes a message as today's `--transports` does. A reply that is not a 2xx fails the hook.
- **`janus verify` asks for messages as today's verifier does** when its configuration names no
  `produce-message` hook, and takes `--transports NAME:PORT[/PATH]`; an `http` entry is where HTTP
  interactions go.
- **A transport names its own parts.** Its contribution declares `exchange-parts` (`initiating`,
  `answering`), and the engine spells no part name of its own. A served request that does not match
  is completed with `transport/dispose`, disposition `reject`, whose new `details` carry the
  mismatches; the HTTP transport still answers it with a 500 naming them. `TransportComponent` has a
  new required method, `exchange_parts`.
- **`verification/explain` shows a message's plan**, rooted at the `message` part, with the new legacy
  action `legacy:message-type`.
