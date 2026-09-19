# bindings

The binding-generation pipeline (plan task 6.1): layer 1 of every Janus SDK — typed views of the spec
schemas (SDK specification §2) — produced by a command rather than by hand.

```bash
cargo run -p pact_janus_bindings -- generate                    # both SDKs
cargo run -p pact_janus_bindings -- generate --only typescript  # or --only jvm
cargo run -p pact_janus_bindings -- stage                       # normalise and prepare only
```

Needs Node (with `npm ci` done in `sdks/typescript/`) and JDK 17. Exit code 2 with the problems on
stderr when a schema cannot be turned into bindings.

## What it does

1. **Reads [`sdks/bindings.json`](../../sdks/bindings.json)**: the schema version directories an SDK
   is built from and what each language calls the result (a TypeScript module, a Java package).
2. **Normalises each set** into `target/bindings/schemas/<set>/`, one standalone schema per type.
   The spec schemas are written for people and for `schema-compat`: several types to a file under
   `$defs`, files that are pure `$defs` containers, `"$ref": "#"` recursion and references between
   files. The generators handle that unevenly (jsonschema2pojo generates nothing from a container
   file and names classes after files), so every type becomes its own file named by its title —
   a non-container root, a `$defs` entry, or a titled subschema nested in either — and every
   `$ref` points at the file of the type it names. A `$ref` to anything but a type, or two types
   with one title in a set, is an error: names are what the SDKs are written against.
3. **Prepares each generator's copy** under `target/bindings/{typescript,jvm}/<set>/`, correcting
   the places a generator's reading disagrees with the schema (`prepare_for_typescript`,
   `prepare_for_jvm` in [`src/lib.rs`](src/lib.rs), each tested):
   - *TypeScript*: an annotation-only schema (`Shape.example`) admits any value but was typed as an
     object — it becomes `unknown`; an annotated `"$ref": "#"` was read as an undeclared `Shape1` —
     it is wrapped so the type keeps its name.
   - *JVM*: an untitled open object (`RequestFrame.body`) became an empty class numbered per clash
     (`Contract__1`…) — it becomes `Map<String, V>`; and `default` is dropped, because jsonschema2pojo
     initialises fields with it and would write `"last": false` into a document that never said so.
     (`initializeCollections` is off in the Gradle config for the same reason: an absent optional
     array must not come back as `[]`.)
4. **Runs each generator** over its copy, into a cleared output tree: `npm run generate:bindings` in
   `sdks/typescript` (json-schema-to-typescript, one type at a time, concatenated per set) and
   `./gradlew :bindings:generateBindings` in `sdks/jvm` (jsonschema2pojo). Neither step changes a
   schema: every change is made in step 2 or 3, here, in tested Rust.
5. **Renders the open vocabularies.** Neither generator does anything with `x-known-values`
   (engine-protocol spec §2.2 rule 1), so an SDK would otherwise hand-copy operation names and error
   codes out of the specs. Each one becomes a constant named after its type and member
   (`RequestFrame.op` → `RequestFrameOp`): an `as const` object in `<set>.vocabulary.ts`, a nested
   class of `Vocabulary` in Java with a `KNOWN` list. They are *known* values of an *open*
   vocabulary — an SDK compares against them, it never rejects a value outside them.

Every generated file carries a header naming this command. CI (`bindings` job) runs `generate` and
fails on any difference from what is checked in: bindings that do not match their schemas are a
second source of truth.

## Known limits

- Property order in generated code is alphabetical, not the schema's: `serde_json` sorts object keys
  unless its `preserve_order` feature is on, and turning it on here would turn it on for the whole
  workspace build, kernel included. JSON members are unordered, so nothing but reading order changes.
- Conditional and combinator structure (`Frame`'s `if`/`then` per `type`, `ResponseFrame`'s
  ok-xor-error `oneOf`) does not survive into Java — jsonschema2pojo ignores it — and survives only
  partially in TypeScript. The member types are right; the cross-member constraints are the engine's
  to enforce, and it does.
- Types are not shared across sets: `contract.InteractionSpec.parts` holds `ShapePart` maps, not
  `shape.Shape` values, because the contract schema does not reference the shape schema. The SDK's
  idiomatic layer puts a `Shape` there; the types do not say so.
