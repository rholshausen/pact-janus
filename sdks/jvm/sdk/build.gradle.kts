// The idiomatic layer of the Janus JVM SDK (plan task 6.3): the DSL every primitive of
// Documentation/specs/sdk-specification/behavioural-spec.json names, the subprocess embedding of
// the engine, and the JUnit Jupiter integration. Hand-written; the protocol's own types come from
// `:bindings`, which is generated and never edited. See ../STYLE.md.

plugins {
  `java-library`
}

java {
  toolchain {
    languageVersion = JavaLanguageVersion.of(17)
  }
}

dependencies {
  api(project(":bindings"))
  api(platform("com.fasterxml.jackson:jackson-bom:2.22.4"))
  // Frames are JSON documents; the bindings carry Jackson 2 annotations, so Jackson reads and
  // writes them. Not part of the SDK's public API.
  implementation("com.fasterxml.jackson.core:jackson-databind")

  // The JUnit integration (JanusExtension) compiles against Jupiter's API; a user of it has
  // Jupiter already, and a user who drives `Janus` by hand does not need it.
  compileOnly(platform("org.junit:junit-bom:6.1.3"))
  compileOnly("org.junit.jupiter:junit-jupiter-api")

  testImplementation(platform("org.junit:junit-bom:6.1.3"))
  testImplementation("org.junit.jupiter:junit-jupiter")
  // The integration's own tests launch small JUnit classes and inspect the outcome.
  testImplementation("org.junit.platform:junit-platform-launcher")
}

// The repository root, where the Cargo workspace (and so `target/debug/janus-engine`) lives.
val repoRoot: File = rootProject.projectDir.resolve("../..").canonicalFile
// What `cargo build` calls it on this platform.
val engineName = if (System.getProperty("os.name").startsWith("Windows")) "janus-engine.exe" else "janus-engine"
val engineBinary: File = repoRoot.resolve("target/debug/$engineName")

// Which engine the tests drive (staged plan S0.10), as the TypeScript suite decides it: the one
// JANUS_ENGINE names; the pinned release when JANUS_ENGINE_RELEASE asks for it, fetched and checked
// against its digest, so no Rust toolchain is needed; otherwise this tree's build, which is what a
// change to the engine and an SDK in one commit needs.
val namedEngine: String? = System.getenv("JANUS_ENGINE")?.takeIf { it.isNotEmpty() }
val releasedEngine = namedEngine == null && !System.getenv("JANUS_ENGINE_RELEASE").isNullOrEmpty()

// The end-to-end tests drive the real engine, so they must never run against a stale one: cargo is
// incremental, so asking it every time costs a no-op check when nothing changed.
val buildEngine = tasks.register<Exec>("buildEngine") {
  group = "build"
  description = "Builds the janus-engine subprocess binary the end-to-end tests run against."
  workingDir = repoRoot
  commandLine("cargo", "build", "-p", "pact_janus_cli", "--bin", "janus-engine")
  outputs.upToDateWhen { false }
}

// The pin every SDK in this repository shares, and where the resolved engine's path is written.
val enginePin: File = repoRoot.resolve("sdks/engine-release.json")
val resolvedEngine = layout.buildDirectory.file("janus-engine/path")

// The SDK runs the engine it was built with when it is told nothing (ADR 0060 decision 6), so the pin
// goes into the jar beside EngineRelease, which reads it. A checkout holding only this directory has
// none to copy (release.yml's sdk-gates job), and names its engine.
tasks.processResources {
  from(enginePin) {
    into("io/pact/janus/sdk/engine")
  }
}

// The SDK's own resolver (EngineRelease), run as the SDK runs it. Asked every time: a cached engine
// whose digest still matches costs a hash, and a cache emptied since is noticed.
val resolveEngine = tasks.register<JavaExec>("resolveEngine") {
  group = "build"
  description = "Resolves the pinned, released janus-engine (sdks/engine-release.json) for the tests."
  classpath = sourceSets["test"].runtimeClasspath
  mainClass = "io.pact.janus.sdk.engine.ResolveEngine"
  args(enginePin.path, resolvedEngine.get().asFile.path)
  inputs.file(enginePin).withPropertyName("enginePin")
  outputs.file(resolvedEngine)
  outputs.upToDateWhen { false }
}

tasks.test {
  useJUnitPlatform()
  // JANUS_ENGINE names the executable, as it does for the rest of the project. The engine is what
  // these tests test: without it as an input Gradle calls the task up to date after an engine
  // change, and the conformance report it leaves behind is the old engine's.
  when {
    namedEngine != null -> {
      environment("JANUS_ENGINE", namedEngine)
      inputs.file(namedEngine).withPropertyName("engineBinary")
    }
    releasedEngine -> {
      dependsOn(resolveEngine)
      val path = resolvedEngine
      inputs.file(enginePin).withPropertyName("enginePin")
      // Known only once resolveEngine has run.
      doFirst { (this as Test).environment("JANUS_ENGINE", path.get().asFile.readText().trim()) }
    }
    else -> {
      dependsOn(buildEngine)
      environment("JANUS_ENGINE", engineBinary.path)
      inputs.file(engineBinary).withPropertyName("engineBinary")
    }
  }
  environment("RUST_LOG", System.getenv("RUST_LOG") ?: "warn")
  // Contracts the end-to-end tests write land under the build directory, never in the source tree.
  systemProperty("janus.test.contracts", layout.buildDirectory.dir("contracts").get().asFile.path)
  // The pin the jar is built with, for the test that checks the jar has it.
  systemProperty("janus.engine.pin", enginePin.path)
  // The shared conformance suite (plan task 6.4): the corpus both SDKs run — from JANUS_SPEC, an
  // unpacked janus-spec.tar.gz, when it names one (staged plan S0.11) — and where this run's report
  // goes for `janus-sdk-gates conformance check` to read.
  val spec = System.getenv("JANUS_SPEC")?.takeIf { it.isNotEmpty() }?.let(::File) ?: repoRoot
  systemProperty("janus.conformance.suite", spec.resolve("conformance").path)
  systemProperty("janus.conformance.report",
      System.getenv("JANUS_CONFORMANCE_REPORT") ?: repoRoot.resolve("target/conformance/jvm.json").path)
  testLogging {
    events("passed", "skipped", "failed")
    exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
  }
}
