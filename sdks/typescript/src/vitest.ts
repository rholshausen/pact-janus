// Vitest integration (STYLE.md, "Test-framework integration"): the suite's one `Janus`, with
// `finalise` registered as an `afterAll` hook — so it runs after the last test, whether or not the
// tests passed, and a withheld contract fails the suite (behavioural spec 'finalise').

import { afterAll } from "vitest";
import { Janus, type JanusConfig } from "./janus.js";

/** Call once per test file, at the top level. */
export function useJanus(config: JanusConfig): Janus {
  const janus = new Janus(config);
  afterAll(async () => {
    await janus.finalise();
  });
  return janus;
}
