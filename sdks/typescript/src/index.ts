// Pact Janus for TypeScript: the idiomatic layer over the generated bindings (SDK spec §2). The
// DSL is the behavioural specification's (Documentation/specs/sdk-specification/
// behavioural-spec.json), spelled the TypeScript way (STYLE.md).

export { Janus, type JanusConfig, type Mock, type Variant, type Closure, type Finalised } from "./janus.js";
export { InteractionBuilder, type RequestParts, type ResponseParts, type MultiValue } from "./interaction.js";
export {
  Shape,
  type Template,
  json,
  integer,
  number,
  decimal,
  string,
  boolean,
  datetime,
  date,
  time,
  regex,
  anyOf,
  oneOf,
  optional,
  nullable,
  eachLike,
} from "./shapes.js";
export { JanusError, VariantsFailedError, ContractWithheldError, type Problem, type VariantFailure } from "./errors.js";
export type { FramePipe } from "./engine/pipe.js";
export type { SubprocessOptions } from "./engine/subprocess.js";
