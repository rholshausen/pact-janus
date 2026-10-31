// The version is gradle.properties' (ADR 0060 decision 2), which knope writes from the change files.
subprojects {
  group = "io.pact.janus"

  repositories {
    mavenCentral()
  }

  // What a release carries (ADR 0060 decision 4): each module as `janus-<project>`, its jar, sources
  // jar, Ivy descriptor and POM, in one flat directory, under the names a GitHub release serves them
  // by. Gradle reads them in place through an `ivy` repository whose pattern is the release's
  // download URL; Maven installs the jar with its POM.
  apply(plugin = "maven-publish")
  apply(plugin = "ivy-publish")
  plugins.withId("java-library") {
    configure<JavaPluginExtension> {
      withSourcesJar()
    }
    configure<PublishingExtension> {
      publications {
        create<IvyPublication>("ivy") {
          module = "janus-${project.name}"
          from(components["java"])
          // A version the Jackson BOM supplied is written as the one it resolved to, so the descriptor
          // needs no BOM, which Ivy cannot import: the BOM's own entry is dropped.
          versionMapping {
            allVariants {
              fromResolutionResult()
            }
          }
          descriptor.withXml {
            val dependencies = asElement().getElementsByTagName("dependency")
            for (index in dependencies.length - 1 downTo 0) {
              val dependency = dependencies.item(index) as org.w3c.dom.Element
              if (dependency.getAttribute("name").endsWith("-bom")) {
                dependency.parentNode.removeChild(dependency)
              }
            }
          }
        }
        // Published nowhere: only its POM is taken (`releaseFiles`).
        create<MavenPublication>("maven") {
          artifactId = "janus-${project.name}"
          from(components["java"])
          versionMapping {
            allVariants {
              fromResolutionResult()
            }
          }
        }
      }
      repositories {
        ivy {
          name = "release"
          url = uri(rootProject.layout.buildDirectory.dir("release/ivy"))
          patternLayout {
            artifact("[module]-[revision](-[classifier]).[ext]")
            ivy("[module]-[revision].ivy.xml")
          }
        }
      }
    }
  }
}

// The files a JVM SDK release carries, in build/release/files: release-sdks.yml attaches them as
// they are.
tasks.register<Sync>("releaseFiles") {
  group = "publishing"
  description = "Writes the files a JVM SDK release carries (ADR 0060 decision 4) to build/release/files."
  val modules = listOf("bindings", "sdk")
  dependsOn(modules.map { ":$it:publishIvyPublicationToReleaseRepository" })
  dependsOn(modules.map { ":$it:generatePomFileForMavenPublication" })
  into(layout.buildDirectory.dir("release/files"))
  from(layout.buildDirectory.dir("release/ivy")) {
    // The checksums Gradle writes beside each file: the release's SHA256SUMS covers every file.
    exclude("**/*.md5", "**/*.sha1", "**/*.sha256", "**/*.sha512")
  }
  for (module in modules) {
    val pom = "janus-$module-$version.pom"
    from(project(":$module").layout.buildDirectory.file("publications/maven/pom-default.xml")) {
      rename { pom }
    }
  }
}
