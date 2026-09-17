// A scripted `before-request` hook (lifecycle-hooks spec §9): stamp every replayed request with a
// correlation id, so a provider's own logs can be lined up with the verification run that caused
// them.
//
// Everything this hook needs arrives in `ctx` — there is no `require`, no `fetch`, no environment
// and no clock beyond the language's own, because a bare interpreter has no ambient capabilities
// and the loader is what fills in `ctx.config`.
function hook(ctx) {
  const headers = janus.json(ctx.parts.request.headers) || {};
  const prefix = (ctx.config && ctx.config.prefix) || "janus";
  const variant = ctx.variant ? ctx.variant.id : "unknown";

  headers["x-correlation-id"] = [prefix + "/" + ctx.exchange.id + "/" + variant];
  janus.log("debug", "stamped " + headers["x-correlation-id"][0]);

  return {
    outcome: "ok",
    changes: { "parts.request.headers": janus.slot(headers) },
  };
}
