// Generated protocol bindings (plan task 6.1): jsonschema2pojo POJOs for the spec schema sets
// sdks/bindings.json names, plus a Vocabulary class per set. Everything under src/main is
// generated — regenerate with `cargo run -p pact_janus_bindings -- generate` from the repository
// root, which normalises the schemas first and then runs `generateBindings` below over them.

import java.io.File
import java.io.FileFilter
import org.jsonschema2pojo.AnnotationStyle
import org.jsonschema2pojo.DefaultGenerationConfig
import org.jsonschema2pojo.Jsonschema2Pojo
import org.jsonschema2pojo.NoopRuleLogger
import org.jsonschema2pojo.SourceType

buildscript {
  repositories {
    mavenCentral()
  }
  dependencies {
    classpath("org.jsonschema2pojo:jsonschema2pojo-core:1.3.3")
  }
}

plugins {
  `java-library`
}

java {
  toolchain {
    languageVersion = JavaLanguageVersion.of(17)
  }
}

dependencies {
  // The POJOs carry Jackson 2 annotations (their additionalProperties map is what keeps a newer
  // engine's members through a round trip — spike 1.1 finding 14), so the annotations are API.
  api(platform("com.fasterxml.jackson:jackson-bom:2.22.2"))
  api("com.fasterxml.jackson.core:jackson-annotations")

  testImplementation("com.fasterxml.jackson.core:jackson-databind")
  testImplementation(platform("org.junit:junit-bom:6.1.3"))
  testImplementation("org.junit.jupiter:junit-jupiter")
  testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
  useJUnitPlatform()
  systemProperty("janus.specs", rootProject.projectDir.resolve("../../Documentation/specs").canonicalPath)
}

class BindingsConfig(private val schemas: File, private val out: File, private val pkg: String) :
  DefaultGenerationConfig() {
  override fun getSource() = listOf(schemas.toURI().toURL()).iterator()
  override fun getTargetDirectory() = out
  override fun getTargetPackage() = pkg
  override fun getTargetVersion() = "17"
  override fun getSourceType() = SourceType.JSONSCHEMA
  override fun getAnnotationStyle() = AnnotationStyle.JACKSON2
  override fun getFileFilter() = FileFilter { it.name.endsWith(".json") }
  // The normalised schemas are named by title already; this keeps inline titled members the same.
  override fun isUseTitleAsClassname() = true
  // Sequence numbers and sizes are JSON integers with no declared bound.
  override fun isUseLongIntegers() = true
  // An absent optional array must stay absent on the way back out, not become `[]`: for
  // `UpgradeFindings` the empty list is itself a claim (contract spec §8.4).
  override fun isInitializeCollections() = false
  // `javax.annotation.Generated` would drag a dependency in for a comment; the file header says it.
  override fun isIncludeGeneratedAnnotation() = false
}

val bindingsSets = providers.gradleProperty("bindingsSets")
val generatedSources = layout.projectDirectory.dir("src/main/java").asFile

tasks.register("generateBindings") {
  group = "build"
  description = "Generates POJOs from the normalised schema sets (run via tools/bindings, not directly)."
  // Runs once per schema change, by hand; the config class it hands jsonschema2pojo is a script
  // class, which the configuration cache cannot store.
  notCompatibleWithConfigurationCache("drives jsonschema2pojo with a build-script generation config")
  doLast {
    val setsFile = File(bindingsSets.orNull ?: throw GradleException("-PbindingsSets=<target/bindings/sets.json> is required"))
    @Suppress("UNCHECKED_CAST")
    val sets = (groovy.json.JsonSlurper().parse(setsFile) as Map<String, Any>)["sets"] as List<Map<String, Any>>
    for (set in sets) {
      @Suppress("UNCHECKED_CAST")
      val schemas = (set["schemas"] as Map<String, String>).getValue("jvm")
      Jsonschema2Pojo.generate(
        BindingsConfig(File(schemas), generatedSources, set["jvm"] as String),
        NoopRuleLogger(),
      )
    }
  }
}
