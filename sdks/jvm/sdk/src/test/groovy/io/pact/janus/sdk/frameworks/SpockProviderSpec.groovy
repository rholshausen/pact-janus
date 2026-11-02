package io.pact.janus.sdk.frameworks

import io.pact.janus.sdk.Verifier
import spock.lang.AutoCleanup
import spock.lang.Requires
import spock.lang.Shared
import spock.lang.Specification

/** A provider test under Spock: one verification, its states set up by closures. */
@Requires({ sys["janus.harness"] == "true" })
class SpockProviderSpec extends Specification {
  @Shared @AutoCleanup
  UserService service = UserService.start()

  def "satisfies UserConsumer's pact"() {
    when:
    def result = Verifier.forProvider(service.url())
        .withPacts([UserService.PACT])
        .state("users exist") { service.aliceExists() }
        .state("a user with ID 1 exists") { service.aliceExists() }
        .verify()

    then:
    result.status() == "verified"
  }
}
