// Copied from pact-jvm at 1928f2247 (4.7.8), consumer/src/main/java/.../dsl/LambdaDslJsonBody.java, its package and
// imports changed to io.pact.janus.facade. (ADR 0070).
package io.pact.janus.facade.consumer.dsl;

public class LambdaDslJsonBody extends LambdaDslObject {

    private final PactDslJsonBody dslPart;

    LambdaDslJsonBody(final PactDslJsonBody dslPart) {
        super(dslPart);
        this.dslPart = dslPart;
    }

    public DslPart build() {
      dslPart.close();
      return dslPart;
    }
}
