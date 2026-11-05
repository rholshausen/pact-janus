// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.support.expressions.DataType;
import io.pact.janus.facade.internal.RegexExample;
import java.util.List;

/** A request with its path. */
public class PactDslRequestWithPath extends PactDslRequestBase<PactDslRequestWithPath> {
  String path;

  PactDslRequestWithPath(PactDslRequestWithoutPath request, String path) {
    super(request.consumerPactBuilder, request.consumerName, request.providerName, request.state, request.description,
        request.defaultRequestValues, request.defaultResponseValues, request.version);
    copyFrom(request);
    this.path = path;
  }

  /** The next interaction's request, after {@code existing}'s response, as pact-jvm starts one. */
  PactDslRequestWithPath(PactDslRequestWithPath existing, String description,
      java.util.List<io.pact.janus.facade.core.model.ProviderState> state) {
    super(existing.consumerPactBuilder, existing.consumerName, existing.providerName, state, description,
        existing.defaultRequestValues, existing.defaultResponseValues, existing.version);
    this.path = existing.path;
    setupDefaultValues();
  }

  public String getPath() {
    return path;
  }

  public String getDescription() {
    return description;
  }

  public PactSpecVersion getVersion() {
    return version;
  }

  public PactDslRequestWithPath path(String path) {
    this.path = path;
    return this;
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
    this.path = path;
    return this;
  }

  public PactDslRequestWithPath pathFromProviderState(String expression, String example) {
    requestGenerators.addGenerator(Category.PATH, "", new ProviderStateGenerator(expression, DataType.STRING));
    path = example;
    return this;
  }

  public PactDslResponse willRespondWith() {
    return new PactDslResponse(consumerPactBuilder, this, defaultRequestValues, defaultResponseValues, version);
  }

  List<io.pact.janus.facade.core.model.ProviderState> states() {
    return state;
  }

  String consumer() {
    return consumerName;
  }

  String provider() {
    return providerName;
  }
}
