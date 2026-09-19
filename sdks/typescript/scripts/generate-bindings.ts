// The TypeScript leg of the binding-generation pipeline (plan task 6.1). Run by
// `cargo run -p pact_janus_bindings -- generate`, never directly: that command normalises the spec
// schemas first (one standalone schema per type, prepared for this generator — see tools/bindings)
// and passes the description of what it staged. This script only turns each staged set into one
// module with json-schema-to-typescript; it changes no schema.
//
//   generate-bindings.ts <target/bindings/sets.json> <out dir>

import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { compile, type JSONSchema } from "json-schema-to-typescript";

interface StagedSet {
  name: string;
  schemas: { typescript: string };
  types: string[];
  typescript: string;
}

const [setsFile, outDir] = process.argv.slice(2);
if (!setsFile || !outDir) {
  throw new Error("usage: generate-bindings.ts <sets.json> <out dir>");
}

const { header, sets } = JSON.parse(await readFile(setsFile, "utf8")) as { header: string; sets: StagedSet[] };

for (const set of sets) {
  // Each type is compiled on its own, with the types it refers to left as bare names: every one of
  // them is another type of the same set, so concatenating the set declares each exactly once.
  // (Compiling the set through one root instead makes the generator meet a type twice — once via
  // the root, once via a reference — and number the second meeting: `Frame1`, `Event1`.)
  const declarations = [];
  for (const type of set.types) {
    const schema = JSON.parse(await readFile(join(set.schemas.typescript, `${type}.json`), "utf8")) as JSONSchema;
    declarations.push(
      await compile(schema, type, {
        cwd: set.schemas.typescript,
        bannerComment: "",
        declareExternallyReferenced: false,
        additionalProperties: true,
        strictIndexSignatures: false,
      }),
    );
  }
  await writeFile(join(outDir, `${set.typescript}.ts`), `${header}\n${declarations.join("\n")}`);
}
