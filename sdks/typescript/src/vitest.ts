// Vitest integration (STYLE.md, "Test-framework integration"): the suite's one `Janus`, with
// `finalise` registered as an `afterAll` hook — so it runs after the last test, whether or not the
// tests passed, and a withheld contract fails the suite (behavioural spec 'finalise').

import { afterAll, expect, test } from "vitest";
import type { InteractionBuilder } from "./interaction.js";
import { Janus, type Closure, type JanusConfig, type MessageClosure } from "./janus.js";
import type { MessageBuilder } from "./message.js";
import { ShapeRecorder, testSource, type ShapeRecorderConfig } from "./recorder.js";

/** The suite's one `Janus`, which can also make each variant a test of its own. */
export class VitestJanus extends Janus {
  /**
   * `execute` as one test per variant the engine selects, each named `label [id]` (`base` for the
   * base variant), registered when this resolves: await it at the top level of a test file, or
   * inside an async `describe`. A failing variant fails only its own test, with its closure's error
   * or the engine's verdict, and the engine withholds the contract.
   */
  async variants(interaction: InteractionBuilder, closure: Closure): Promise<void>;
  async variants(message: MessageBuilder, closure: MessageClosure): Promise<void>;
  async variants(interaction: InteractionBuilder | MessageBuilder, closure: Closure | MessageClosure): Promise<void> {
    const tests = await this.variantTests(interaction as InteractionBuilder, closure as Closure);
    for (const variant of tests) {
      test(variant.name, () => variant.run());
    }
  }
}

/** Call once per test file, at the top level. */
export function useJanus(config: JanusConfig): VitestJanus {
  const janus = new VitestJanus(config);
  afterAll(async () => {
    await janus.finalise();
  });
  return janus;
}

/** A shape recorder for this test file, at its top level: the file is its source, and it finishes after its tests. */
export function useShapeRecorder(config: ShapeRecorderConfig): ShapeRecorder {
  const source = testSource(expect.getState().testPath);
  const recorder = new ShapeRecorder({ ...(source === undefined ? {} : { source }), ...config });
  afterAll(async () => {
    await recorder.finish();
  });
  return recorder;
}
