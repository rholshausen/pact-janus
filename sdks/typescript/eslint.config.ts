import js from "@eslint/js";
import { defineConfig } from "eslint/config";
import tseslint from "typescript-eslint";

export default defineConfig(
  // Generated bindings are never hand-edited, so never hand-linted either: a finding there is a
  // finding about the generator or the schema (tools/bindings).
  { ignores: ["dist/", "src/generated/"] },
  js.configs.recommended,
  tseslint.configs.strict,
);
