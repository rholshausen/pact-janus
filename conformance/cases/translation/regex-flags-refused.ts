// translation/regex-flags-refused, in TypeScript: a RegExp's flags are out of band, since a shape's
// pattern is the RegExp's source alone.
import { regex } from "pact-janus";

export async function patternWithFlags(): Promise<void> {
  regex(/ord-\d+/i, "ORD-42");
}

export async function patternWithoutFlags(): Promise<void> {
  regex(/ord-\d+/, "ord-42");
}
