// pact-jvm's consumer DSL for JUnit 5, over the engine (staged plan S2.5b, ADR 0070): its builders
// port pact-jvm's Java DSL, building the example, rules and generators pact-jvm builds, and its
// extension submits them as `add-pact-interaction` calls. Hand-written, in the `facade` layer of
// ../thinness.json. A suite moves to it by replacing `au.com.dius.pact.` with
// `io.pact.janus.facade.` in its imports.

plugins {
  `java-library`
}

java {
  toolchain {
    languageVersion = JavaLanguageVersion.of(17)
  }
}

dependencies {
  api(project(":sdk"))
  // The facade writes the frames' bodies and the examples' JSON. Not part of its public API.
  implementation(platform("com.fasterxml.jackson:jackson-bom:2.22.4"))
  implementation("com.fasterxml.jackson.core:jackson-databind")

  // The JUnit 5 extension compiles against Jupiter's API; a user of it has Jupiter already.
  compileOnly(platform("org.junit:junit-bom:6.1.3"))
  compileOnly("org.junit.jupiter:junit-jupiter-api")

  testImplementation(platform("org.junit:junit-bom:6.1.3"))
  testImplementation("org.junit.jupiter:junit-jupiter")
  testImplementation("org.junit.platform:junit-platform-launcher")
  // What pact-jvm's own consumer tests use, which run here with their imports changed.
  testImplementation("org.apache.httpcomponents.client5:httpclient5-fluent:5.5")
  testImplementation("org.hamcrest:hamcrest:3.0")
  testImplementation("commons-io:commons-io:2.20.0")
  testImplementation("org.apache.commons:commons-collections4:4.5.0")
  testImplementation("org.apache.groovy:groovy-json:4.0.33")
  testImplementation("io.rest-assured:rest-assured:5.5.6")
  testImplementation("com.squareup.okhttp3:okhttp:4.12.0")
  testImplementation(platform("com.fasterxml.jackson:jackson-bom:2.22.4"))
  testImplementation("com.fasterxml.jackson.core:jackson-databind")
}

// The engine the tests drive is the one :sdk's tests drive: JANUS_ENGINE, else the pinned release
// when JANUS_ENGINE_RELEASE asks for it, else this tree's build.
val sdk = project(":sdk")
val repoRoot: File = rootProject.projectDir.resolve("../..").canonicalFile
val engineName = if (System.getProperty("os.name").startsWith("Windows")) "janus-engine.exe" else "janus-engine"
val engineBinary: File = repoRoot.resolve("target/debug/$engineName")
val namedEngine: String? = System.getenv("JANUS_ENGINE")?.takeIf { it.isNotEmpty() }
val releasedEngine = namedEngine == null && !System.getenv("JANUS_ENGINE_RELEASE").isNullOrEmpty()

// The contracts the ported tests write, cleared before they run, since pact-jvm's tests merge into
// what is there and a contract from an older build would conflict.
val pacts = layout.buildDirectory.dir("pacts")

tasks.test {
  useJUnitPlatform {
    excludeTags("contracts")
  }
  val pactsDir = pacts.get().asFile
  doFirst { pactsDir.deleteRecursively() }
  filter {
    // pact-jvm's tests that wait on an engine item, each with the item that closes it. Delete the line
    // when the item lands, and the test runs.
    // S1.38: 100.0 is a v1-v4 decimal by how it is written.
    excludeTestsMatching("io.pact.janus.facade.consumer.junit5.OkHttpTest")
  }
  when {
    namedEngine != null -> {
      environment("JANUS_ENGINE", namedEngine)
      inputs.file(namedEngine).withPropertyName("engineBinary")
    }
    releasedEngine -> {
      dependsOn(":sdk:resolveEngine")
      val path = sdk.layout.buildDirectory.file("janus-engine/path")
      doFirst { (this as Test).environment("JANUS_ENGINE", path.get().asFile.readText().trim()) }
    }
    else -> {
      dependsOn(":sdk:buildEngine")
      environment("JANUS_ENGINE", engineBinary.path)
      inputs.file(engineBinary).withPropertyName("engineBinary")
    }
  }
  environment("RUST_LOG", System.getenv("RUST_LOG") ?: "warn")
  // pact-jvm's own property for where pacts go, which the facade reads for its contracts.
  systemProperty("pact.rootDir", pacts.get().asFile.path)
  // The pacts pact-jvm 4.7.8 wrote running these tests, which each contract is compared with.
  systemProperty("janus.facade.pacts", layout.projectDirectory.dir("src/test/resources/pact-jvm").asFile.path)
  testLogging {
    events("passed", "skipped", "failed")
    exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
  }
}

// S2.5b's exit, after the ported tests have written their contracts: each is what `janus upgrade`
// writes from the pact pact-jvm wrote running the same test.
val compareContracts = tasks.register<Test>("compareContracts") {
  description = "Compares each contract the ported pact-jvm tests wrote with janus upgrade of pact-jvm's pact."
  group = "verification"
  testClassesDirs = sourceSets["test"].output.classesDirs
  classpath = sourceSets["test"].runtimeClasspath
  useJUnitPlatform {
    includeTags("contracts")
  }
  shouldRunAfter(tasks.test)
  dependsOn(tasks.test)
  val engine = tasks.test.get().environment["JANUS_ENGINE"]
  if (engine != null) {
    environment("JANUS_ENGINE", engine)
  }
  systemProperty("pact.rootDir", pacts.get().asFile.path)
  systemProperty("janus.facade.pacts", layout.projectDirectory.dir("src/test/resources/pact-jvm").asFile.path)
  testLogging {
    events("passed", "skipped", "failed")
    exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
  }
}

tasks.check {
  dependsOn(compareContracts)
}
