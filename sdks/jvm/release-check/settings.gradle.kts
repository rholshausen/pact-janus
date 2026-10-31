// A project outside the SDK that resolves a JVM SDK release and uses it as a user would (staged plan
// S1.36, ADR 0060 decision 7). release-sdks.yml copies it out of the repository and runs it with the
// SDK's own Gradle wrapper: `gradlew -p <copy> test -PjanusVersion=<version> -PjanusRelease=<url>`.
// It is not part of the SDK's build.
rootProject.name = "pact-janus-release-check"
