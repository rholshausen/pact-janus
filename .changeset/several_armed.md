---
engine: minor
---

# Several interactions armed at once, each for every call

A transport now holds every interaction armed on it, and an arming lasts until its variant is
reported, the handle is armed again, or the session ends (ADR 0069). Each arrival is matched against
every arming and goes to one it matches, the most specific first: the one whose match passed the most
plan actions, then the first armed. An arrival that matches none fails the arming it came closest to,
and is rejected as a mismatch is now. Every call an arming takes is one more exchange of its variant,
which fails if any of them failed. So a test that arms several interactions and then runs its client,
or calls an interaction more than once, is served as today's mock server serves it. A host that
served and called one variant at a time sees no difference, but for a second call under one variant,
which is now served where it was refused.
