// Pact Janus JVM SDK prototype (plan Phase 6). `bindings` holds only generated code (plan task
// 6.1); `sdk` is the idiomatic layer (task 6.3) — DSL, engine embedding and JUnit integration —
// written from the SDK specification (Documentation/specs/sdk-specification/).
rootProject.name = "pact-janus-jvm"

include("bindings")
include("sdk")
// pact-jvm's consumer DSL over the engine (ADR 0070), its own jar so a user of the Janus DSL does not get it.
include("facade")
