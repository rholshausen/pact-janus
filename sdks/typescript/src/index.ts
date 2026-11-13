// Pact Janus for TypeScript: the idiomatic layer over the generated bindings (SDK spec §2). The
// DSL is the behavioural specification's (Documentation/specs/sdk-specification/
// behavioural-spec.json), spelled the TypeScript way (STYLE.md).

export {
  Janus,
  type JanusConfig,
  type ComponentDeclaration,
  type Redaction,
  type Mock,
  type Variant,
  type Closure,
  type MessageClosure,
  type Finalised,
} from "./janus.js";
export { MessageBuilder, ReceivedMessage, type MessageSlots } from "./message.js";
export {
  InteractionBuilder,
  VariantBinding,
  whenVariant,
  variantCases,
  type RequestParts,
  type ResponseParts,
  type MultiValue,
  type HeaderValue,
  type Spellable,
  type ExcludedPoints,
} from "./interaction.js";
export {
  Shape,
  Content,
  HeaderList,
  type Template,
  json,
  content,
  headerList,
  pathTemplate,
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
  forbidden,
  eachLike,
  generated,
} from "./shapes.js";
export {
  ShapeRecorder,
  type ShapeRecorderConfig,
  type Operation,
  type RecordedResponse,
  type ObserveOptions,
  type MessageOperation,
  type ObserveMessageOptions,
} from "./recorder.js";
export {
  Verifier,
  type VerifierConfig,
  type SourceOptions,
  type StateHandler,
  type StateSetup,
  type RequestFilter,
  type Message,
  type MessageProducer,
} from "./verifier.js";
export {
  JanusError,
  VariantsFailedError,
  ContractWithheldError,
  VerificationFailedError,
  StateUnsupportedError,
  type Problem,
  type VariantFailure,
  type VerificationResult,
  type VerificationResultEntry,
} from "./errors.js";
export type { FramePipe } from "./engine/pipe.js";
export type { SubprocessOptions } from "./engine/subprocess.js";
export { EngineNotStartedError } from "./engine/release.js";
