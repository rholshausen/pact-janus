// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.consumer.ConsumerPactBuilder;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.ProviderState;
import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.support.expressions.DataType;
import io.pact.janus.facade.internal.RegexExample;
import java.util.List;

/** A request before its path. */
public class PactDslRequestWithoutPath extends PactDslRequestBase<PactDslRequestWithoutPath> {
  public PactDslRequestWithoutPath(ConsumerPactBuilder consumerPactBuilder, String consumerName, String providerName,
      List<ProviderState> state, String description, PactDslRequestWithoutPath defaultRequestValues,
      PactDslResponse defaultResponseValues, PactSpecVersion version) {
    super(consumerPactBuilder, consumerName, providerName, state, description, defaultRequestValues,
        defaultResponseValues, version);
    setupDefaultValues();
  }

  /** As a builder of default request values, with nothing of an interaction. */
  public PactDslRequestWithoutPath() {
    super(null, "", "", List.of(), "", null, null, PactSpecVersion.V3);
  }

  public PactDslRequestWithPath path(String path) {
    return new PactDslRequestWithPath(this, path);
  }

  /** A path matching {@code pathRegex}, its example drawn from the pattern the same way every run (ADR 0070 decision 3). */
  public PactDslRequestWithPath matchPath(String pathRegex) {
    return matchPath(pathRegex, RegexExample.of(pathRegex));
  }

  public PactDslRequestWithPath matchPath(String pathRegex, String path) {
    if (!path.matches(pathRegex)) {
      throw new InvalidMatcherException("Example \"" + path + "\" does not match regular expression \"" + pathRegex + "\"");
    }
    requestMatchers.addCategory("path").addRule(new RegexMatcher(pathRegex, null));
    return new PactDslRequestWithPath(this, path);
  }

  public PactDslRequestWithPath pathFromProviderState(String expression, String example) {
    requestGenerators.addGenerator(Category.PATH, "", new ProviderStateGenerator(expression, DataType.STRING));
    return new PactDslRequestWithPath(this, example);
  }
}
