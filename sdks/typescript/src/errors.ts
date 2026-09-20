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

/** One variant whose closure threw. */
export interface VariantFailure {
  variant: protocol.VariantDescriptor;
  cause: unknown;
}

/** `execute` ran every selected variant, and at least one closure threw. */
export class VariantsFailedError extends Error {
  readonly interaction: string;
  readonly failures: VariantFailure[];
  readonly selected: number;

  constructor(interaction: string, failures: VariantFailure[], selected: number) {
    const lines = failures.map(({ variant, cause }) => {
      const reason = cause instanceof Error ? `${cause.name}: ${cause.message}` : String(cause);
      return `  ✗ ${label(variant)}\n      ${reason.split("\n").join("\n      ")}`;
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
 * `finalise` wrote no contract: the engine did not verify every interaction, or a test in this
 * suite failed — the engine verified the exchanges, but only the closure knows whether the consumer
 * handled what it was sent, and a contract must not claim a variant its own test failed on.
 */
export class ContractWithheldError extends Error {
  readonly results: protocol.InteractionResult[];
  /** Descriptions of the interactions whose `execute` failed in this suite. */
  readonly failedTests: string[];
  /** Whether the engine itself returned no contract, as against the SDK withholding one it did. */
  readonly engineWithheld: boolean;

  constructor(
    results: protocol.InteractionResult[],
    descriptions: Map<string, string>,
    failedTests: string[],
    engineWithheld: boolean,
  ) {
    const lines: string[] = [];
    for (const result of results.filter((r) => r.status !== "verified")) {
      lines.push(`  ${descriptions.get(result.handle) ?? result.handle}: ${result.status}`);
      for (const variant of result.variants ?? []) {
        if (variant.status === "verified") {
          continue;
        }
        lines.push(`    ✗ ${variant.variant}: ${variant.status}`);
        for (const mismatch of (variant.mismatches ?? []) as Array<Record<string, unknown>>) {
          lines.push(`        ${String(mismatch.path ?? "")} ${String(mismatch.message ?? JSON.stringify(mismatch))}`);
        }
      }
    }
    for (const description of failedTests) {
      lines.push(`  ${description}: its test failed`);
    }
    super(`No contract written:\n${lines.join("\n")}`);
    this.name = "ContractWithheldError";
    this.results = results;
    this.failedTests = failedTests;
    this.engineWithheld = engineWithheld;
  }
}

/** `label (id)`, or just the id when the engine gave no separate label. */
export function label(variant: protocol.VariantDescriptor): string {
  const name = typeof variant.label === "string" ? variant.label : undefined;
  return name && name !== variant.id ? `${name} (${variant.id})` : variant.id;
}
