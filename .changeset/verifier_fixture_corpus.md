---
default: patch
---

# `janus-spec.tar.gz` carries the verifier fixture corpus

The specification archive gains `corpora/verifier/`: pacts with the providers that answer them, and
`disagreements.json`, every verdict on which `janus verify` and today's `pact_verifier_cli` (1.3.5)
are known to disagree, each with the plan item that accounts for it. The fixtures include the
pact-compatibility-suite's HTTP provider scenarios, V1 to V4 (Apache-2.0). Nothing an SDK runs today
reads it. A provider integration (staged plan S1.15) will be held to it.

What the ledger says about this release, for anyone deciding whether `janus verify` can replace
today's verifier yet: plain-text, XML, form, multipart and binary bodies are not verified (S1.6); a
pending interaction fails the run (S1.3); a repeated response header is judged as one list (S3.3);
and a regex matches anywhere in a value, where today's verifiers require the whole value to match
(S1.18).
