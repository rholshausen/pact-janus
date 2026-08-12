// Bindings round, TS leg: strict-mode usage of the generated types — compile-time envelope
// checking, open vocabulary classification, and unknown-member access via the index signature.
import type { EngineErrorV1OpenWorld as EngineError } from "./engine-error-v1";

const KNOWN_CODES = [
  "invalid-spec",
  "unsupported-spec-version",
  "session-not-found",
  "internal",
] as const;

function classify(err: EngineError): string {
  return (KNOWN_CODES as readonly string[]).includes(err.code)
    ? `known code: ${err.code}`
    : `unknown code '${err.code}' — policy: warn`;
}

const frame: EngineError = JSON.parse(
  '{"code":"component-unavailable","message":"x","retryable":true}',
);
const retryable: unknown = frame["retryable"]; // unknown members typed as `unknown`, not lost
console.log(classify(frame), retryable, JSON.stringify(frame));

// Compile-time safety retained for the closed envelope:
// @ts-expect-error message is required
const bad: EngineError = { code: "internal" };
void bad;
