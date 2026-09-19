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
  api(platform("com.fasterxml.jackson:jackson-bom:2.22.2"))
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
val engineBinary: File = repoRoot.resolve("target/debug/janus-engine")

// The end-to-end tests drive the real engine, so they must never run against a stale one: cargo is
// incremental, so asking it every time costs a no-op check when nothing changed.
val buildEngine = tasks.register<Exec>("buildEngine") {
  group = "build"
  description = "Builds the janus-engine subprocess binary the end-to-end tests run against."
  workingDir = repoRoot
  commandLine("cargo", "build", "-p", "pact_janus_cli", "--bin", "janus-engine")
  outputs.upToDateWhen { false }
}

tasks.test {
  useJUnitPlatform()
  dependsOn(buildEngine)
  // JANUS_ENGINE names the executable, as it does for the rest of the project; an explicit value
  // in the environment wins, so a developer can point the suite at another build.
  environment("JANUS_ENGINE", System.getenv("JANUS_ENGINE") ?: engineBinary.path)
  environment("RUST_LOG", System.getenv("RUST_LOG") ?: "warn")
  // Contracts the end-to-end tests write land under the build directory, never in the source tree.
  systemProperty("janus.test.contracts", layout.buildDirectory.dir("contracts").get().asFile.path)
  testLogging {
    events("passed", "skipped", "failed")
    exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
  }
}
