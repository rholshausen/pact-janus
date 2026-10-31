// Resolves janus-sdk exactly as a user's Gradle build does: through an `ivy` repository whose pattern
// is a GitHub release's download URL (ADR 0060 decision 4). `janusRelease` is that URL, or, before a
// release is published, a directory laid out the same way: `jvm-sdk/v<version>/<file>`.

plugins {
  java
}

java {
  toolchain {
    languageVersion = JavaLanguageVersion.of(17)
  }
}

val janusVersion = providers.gradleProperty("janusVersion").get()
val janusRelease = providers.gradleProperty("janusRelease")
  .getOrElse("https://github.com/rholshausen/pact-janus/releases/download")

repositories {
  mavenCentral()
  ivy {
    url = uri(janusRelease)
    patternLayout {
      artifact("jvm-sdk/v[revision]/[module]-[revision](-[classifier]).[ext]")
      ivy("jvm-sdk/v[revision]/[module]-[revision].ivy.xml")
    }
    content {
      includeGroup("io.pact.janus")
    }
  }
}

dependencies {
  testImplementation("io.pact.janus:janus-sdk:$janusVersion")
  testImplementation(platform("org.junit:junit-bom:6.1.3"))
  testImplementation("org.junit.jupiter:junit-jupiter")
  testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
  useJUnitPlatform()
  // Never up to date: what is under test is the release, which Gradle does not see change.
  outputs.upToDateWhen { false }
  testLogging {
    events("passed", "skipped", "failed")
    showStandardStreams = true
    exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
  }
}
