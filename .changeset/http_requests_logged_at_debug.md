---
default: patch
---

# Every HTTP request to a provider or a broker is logged in full at `debug`

At `--log-level debug` (or `RUST_LOG=debug janus-engine`), the HTTP library logs each request's
URL as `http://host:port/******`. It hides every path below `trace`, because it cannot know which
ones carry a secret. Now the HTTP transport and the broker client log each request themselves:
the method and the whole URL when it is sent, then the status when it is answered, or the error
when nothing answered. That makes debugging which requests ran possible without `trace`.

- **What is shown.** Provider paths come from the contract, and broker URLs are the broker's own
  links, so neither is hidden. User information in a URL (`http://user:password@…`) is removed,
  as everywhere else the engine logs a URL, and headers are not logged.
- **What does not change.** The HTTP library's own lines are still capped at `debug`, so its
  `trace` dump of request bytes, `Authorization` header included, is never logged.
