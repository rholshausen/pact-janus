// How engine outcomes become language-native failures (STYLE.md, "The error surface"). One class per
// kind of failure a user has to act on differently; the engine's own words — code, category,
// details, problems — are carried verbatim, never summarised away (SDK spec §3.2, `errors`).

import type { protocol } from "./generated/index.js";

/** One problem the engine found in a submitted document: where, and what. */
export interface Problem {
  /** A JSON pointer into the interaction specification, e.g. `/parts/response/body`. */
  pointer?: string;
  message: string;
  [k: string]: unknown;
}

/** A structured error from the engine (engine-protocol spec §10), carried verbatim. */
export class JanusError extends Error {
  readonly code: string;
  /** Absent or unknown means `internal` (engine-protocol spec §10.2). */
  readonly category: string;
  readonly details: Record<string, unknown>;
  /** For `interaction-invalid` and `contract-invalid`: every problem the engine found. */
  readonly problems: Problem[];

  constructor(error: protocol.EngineError, context?: string) {
    const problems = Array.isArray(error.details?.problems) ? (error.details.problems as Problem[]) : [];
    const lines = problems.map((p) => `  ${p.pointer ?? "(document)"}: ${p.message}`);
    super([`${context ? `${context}: ` : ""}${error.message} [${error.code}]`, ...lines].join("\n"));
    this.name = "JanusError";
    this.code = error.code;
    this.category = error.category ?? "internal";
    this.details = error.details ?? {};
    this.problems = problems;
  }
}

/** One variant the engine did not verify: its closure threw, or the engine found it failed or unexercised. */
export interface VariantFailure {
  variant: protocol.VariantDescriptor;
  /** The variant's result, as the engine answered the closure's report (ADR 0064). */
  result: protocol.VariantResult;
  /** What the closure threw, when it threw. */
  cause?: unknown;
}

/** `execute` ran every selected variant, and at least one closure threw. */
export class VariantsFailedError extends Error {
  readonly interaction: string;
  readonly failures: VariantFailure[];
  readonly selected: number;

  constructor(interaction: string, failures: VariantFailure[], selected: number) {
    const lines = failures.map(({ variant, result, cause }) => {
      const engine = result.status === "not-exercised" ? "nothing exercised the mock under this variant" : `the engine found this variant ${result.status}`;
      const said = [...(cause === undefined ? engine : causeText(cause)).split("\n"), ...mismatchLines(result.mismatches)];
      return `  ✗ ${label(variant)}\n      ${said.join("\n      ")}`;
    });
    super(`${failures.length} of ${selected} variants failed for '${interaction}':\n${lines.join("\n")}`, {
      cause: failures[0]?.cause,
    });
    this.name = "VariantsFailedError";
    this.interaction = interaction;
    this.failures = failures;
    this.selected = selected;
  }
}

/**
 * `finalise` wrote no contract, because the engine withheld it: an interaction did not verify on a
 * variant it required — its request did not match, nothing exercised it, or its test failed — or the
 * engine refused an interaction the suite added (ADR 0064).
 */
export class ContractWithheldError extends Error {
  readonly results: protocol.InteractionResult[];
  /** The interactions the engine refused when the suite added them. */
  readonly refused: protocol.RefusedInteraction[];
  /** The descriptions of the interactions that did not verify, then of the ones the engine refused. */
  readonly interactions: string[];

  constructor(
    results: protocol.InteractionResult[],
    descriptions: Map<string, string>,
    refused: protocol.RefusedInteraction[],
  ) {
    const failed = results.filter((r) => r.status !== "verified");
    const interactions = [
      ...failed.map((r) => descriptions.get(r.handle) ?? r.handle),
      ...refused.map((r) => r.description ?? "an interaction"),
    ];
    const lines: string[] = [];
    failed.forEach((result, i) => {
      lines.push(`  ${interactions[i]}: ${result.status}`);
      for (const variant of (result.variants ?? []).filter((v) => v.status !== "verified")) {
        const reason = variant.test?.reason === undefined ? [] : [`its test failed: ${variant.test.reason}`];
        lines.push(`    ✗ ${variant.variant}: ${variant.status}`, ...[...reason, ...mismatchLines(variant.mismatches)].map((l) => `        ${l}`));
      }
    });
    refused.forEach((refusal, i) => lines.push(`  ${interactions[failed.length + i]}: refused, ${refusal.message} [${refusal.code}]`));
    super(`No contract written:\n${lines.join("\n")}`);
    this.name = "ContractWithheldError";
    this.results = results;
    this.refused = refused;
    this.interactions = interactions;
  }
}

/** What a closure threw, as text: the engine records it (ADR 0064), and a failure prints it. */
export function causeText(cause: unknown): string {
  return cause instanceof Error ? `${cause.name}: ${cause.message}` : String(cause);
}

function mismatchLines(mismatches: protocol.VariantResult["mismatches"]): string[] {
  return (mismatches ?? []).map((m) => `${m.path === undefined ? "" : `${String(m.path)} `}${String(m.message ?? JSON.stringify(m))}`);
}

/** One interaction × variant result of a provider verification, as `verification/interaction-result` carried it. */
export interface VerificationResultEntry {
  /** The engine's reference to the interaction, as it sent it: its `description` names it. */
  interaction: unknown;
  variant: string;
  status: string;
  pending?: boolean;
  mismatches?: Array<Record<string, unknown>>;
  [k: string]: unknown;
}

/** What `verify` returns, and what its failure carries (behavioural spec 'verify'). */
export interface VerificationResult {
  /** The summary's status: `verified`, `filtered` (not a pass: not every variant was replayed) or `failed`. */
  status: string;
  /** The terminal event's summary document, as the engine sent it. */
  summary: Record<string, unknown>;
  results: VerificationResultEntry[];
  /**
   * Every `verification/warning` payload, in order: what the run noticed and did not fail on, such as
   * a provider state no handler sets up (ADR 0058). The run's report, printed as it ran, says them too.
   */
  warnings: Array<Record<string, unknown>>;
}

/** The run's status was `failed`. Names each failed result that is not pending. */
export class VerificationFailedError extends Error {
  readonly result: VerificationResult;
  readonly failures: VerificationResultEntry[];

  constructor(result: VerificationResult) {
    const failures = result.results.filter((r) => r.status === "failed" && r.pending !== true);
    const lines = failures.flatMap((r) => [
      `  ✗ ${describe(r.interaction)} (${r.variant})`,
      ...(r.mismatches ?? []).map((m) => `      ${String(m.path ?? "")} ${String(m.message ?? JSON.stringify(m))}`),
    ]);
    super(`The provider verification failed${failures.length > 0 ? `:\n${lines.join("\n")}` : ""}`);
    this.name = "VerificationFailedError";
    this.result = result;
    this.failures = failures;
  }
}

function describe(interaction: unknown): string {
  const description = (interaction as { description?: unknown } | null)?.description;
  return typeof description === "string" ? description : JSON.stringify(interaction);
}

/** Thrown by a state handler that cannot put the provider in its state: the variant is state-unavailable. */
export class StateUnsupportedError extends Error {
  constructor(reason: string) {
    super(reason);
    this.name = "StateUnsupportedError";
  }
}

/** `label (id)`, or just the id when the engine gave no separate label. */
export function label(variant: protocol.VariantDescriptor): string {
  const name = typeof variant.label === "string" ? variant.label : undefined;
  return name && name !== variant.id ? `${name} (${variant.id})` : variant.id;
}
