// Run once before the test files: the engine merges what they record into the contract file
// (ADR 0065), so a file an earlier run left would be merged into too, keeping an interaction this run
// changed or removed. A run's contracts are its own output, cleared first, as a build clears its own.
import { rm } from "node:fs/promises";

export default async function clearContracts(): Promise<void> {
  await rm("contracts", { recursive: true, force: true });
}
