// Marks dist/cjs as CommonJS. The SDK's own package.json says "type": "module", so without this
// Node would read the CommonJS build's .js files as ES modules (tsconfig.cjs.json).

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const dir = process.argv[2] ?? "dist/cjs";
mkdirSync(dir, { recursive: true });
writeFileSync(join(dir, "package.json"), `${JSON.stringify({ type: "commonjs" }, null, 2)}\n`);
