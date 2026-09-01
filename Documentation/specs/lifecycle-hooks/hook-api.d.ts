/**
 * The scripted-hook API (v1) — the entire capability surface a hook script has.
 *
 * A script hook is a plain JavaScript function. It is handed the same context document every other
 * hook implementation is handed (`hook-context.schema.json`) and answers with the same result
 * document (design 2.6's `hook.schema.json#/$defs/InvokeResult`); these declarations are the
 * TypeScript view of those two documents, plus the small standard library the runtime binds.
 *
 * There is nothing else. No `require`, no `fetch`, no file system, no environment, no timers — not
 * as a policy the runtime enforces but as a fact of how it is built (ADR 0015, spike 1.6 finding 6):
 * a bare interpreter has no ambient capabilities, so what a script can do is exactly what appears
 * below. Anything a hook needs from the outside world arrives in `ctx.config`, put there by the
 * configuration the loader resolved.
 *
 * TypeScript is a tooling question, not an engine one: the loader transpiles a `.ts` hook and the
 * engine only ever sees JavaScript (spec §9.6).
 */

/** An open string vocabulary: the values below are the ones v1 defines, and a newer engine may pass one that is not. */
type Open<T extends string> = T | (string & {});

export type Point = Open<
  | "before-verification"
  | "state-setup"
  | "before-request"
  | "produce-message"
  | "consume-message"
  | "after-response"
  | "state-teardown"
  | "after-verification"
>;

export type Role = Open<"provider" | "consumer">;

export type Outcome = Open<"ok" | "failed" | "skipped" | "unsupported">;

/** A slot's value, always wrapped — design 2.6 §4. Use `janus.text`/`janus.json`/`janus.bytes` to read one and `janus.slot` to build one. */
export interface SlotValue {
  content?: unknown;
  encoded?: Open<"json" | "text" | "base64">;
  "content-type"?: string;
}

/** Part name -> slot name -> slot value. Part and slot names belong to the transport, never to the kernel. */
export type Parts = Record<string, Record<string, SlotValue>>;

export interface RunContext {
  id?: string;
  consumer?: { name?: string };
  provider?: { name?: string };
  /** Hook name -> the `data` that hook returned at a run point earlier in this run. */
  data?: Record<string, unknown>;
}

export interface InteractionContext {
  description?: string;
  states?: Array<{ name?: string; params?: Record<string, unknown> }>;
  transport?: { kind?: string; mode?: Open<"passive" | "emissive">; [key: string]: unknown };
}

/** Design 2.3's assignment document, unchanged: one entry per active dimension. */
export type Assignment = Array<{ dimension: string; point: string }>;

export interface VariantContext {
  id?: string;
  /** Read the assignment; never parse the id. */
  assignment?: Assignment;
}

export interface StateContext {
  name?: string;
  params?: Record<string, unknown>;
}

export interface ExchangeContext {
  id?: string;
  data?: Record<string, unknown>;
  outcome?: Open<"passed" | "failed" | "state-unavailable" | "not-run">;
}

export interface HookContext {
  point: Point;
  role: Role;
  run?: RunContext;
  interaction?: InteractionContext;
  variant?: VariantContext;
  state?: StateContext;
  exchange?: ExchangeContext;
  parts?: Parts;
  endpoint?: Record<string, unknown>;
  summary?: Record<string, unknown>;
  /** This hook entry's own configuration, already interpolated. Secrets arrive here and nowhere else. */
  config?: Record<string, unknown>;
  /** The dotted context paths this invocation may replace. A `changes` key outside it is refused. */
  mutable?: string[];
  "deadline-ms"?: number;
}

export interface HookResult {
  outcome: Outcome;
  /** Dotted context path -> replacement value. Only declared, permitted paths are applied. */
  changes?: Record<string, unknown>;
  /** Opaque output for later hooks in this run or exchange; reported only when the entry opts in. */
  data?: Record<string, unknown>;
  /** Why it failed, or why the state is unsupported. */
  error?: { code?: string; message: string; details?: Record<string, unknown> };
}

/** The function the runtime calls. Synchronous: a hook that needs to wait for something is an `exec`, `http` or component hook (spec §9.4). */
export type HookFunction = (ctx: HookContext) => HookResult | void;

/** The standard library — the whole of it. */
export interface Janus {
  /** A slot's value as text, whatever it was encoded as. Returns undefined for an absent slot. */
  text(slot: SlotValue | undefined): string | undefined;
  /** A slot's value parsed as JSON, or the value itself when it is already a document. */
  json(slot: SlotValue | undefined): unknown;
  /** A slot's octets, base64-encoded — the form that survives a body that is not text. */
  bytes(slot: SlotValue | undefined): string | undefined;
  /** Build a slot value. `janus.slot("ok")` is text; pass `{ encoded: "base64", contentType }` for octets. */
  slot(value: unknown, options?: { encoded?: SlotValue["encoded"]; contentType?: string }): SlotValue;
  /** The only output a script has. Lines are attributed to the hook and carried as run diagnostics. */
  log(level: Open<"debug" | "info" | "warn" | "error">, message: string): void;
}

declare global {
  const janus: Janus;
}
