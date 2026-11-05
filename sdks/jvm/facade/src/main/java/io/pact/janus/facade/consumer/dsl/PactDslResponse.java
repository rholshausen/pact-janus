// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.consumer.ConsumerPactBuilder;
import io.pact.janus.facade.consumer.xml.PactXmlBuilder;
import io.pact.janus.facade.core.model.BasePact;
import io.pact.janus.facade.core.model.Consumer;
import io.pact.janus.facade.core.model.OptionalBody;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.Provider;
import io.pact.janus.facade.core.model.ProviderState;
import io.pact.janus.facade.core.model.RequestResponseInteraction;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.V4Pact;
import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.matchingrules.ContentTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.MatchingRuleCategory;
import io.pact.janus.facade.core.model.matchingrules.MatchingRules;
import io.pact.janus.facade.core.model.matchingrules.MatchingRulesImpl;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.RuleLogic;
import io.pact.janus.facade.core.model.matchingrules.StatusCodeMatcher;
import io.pact.janus.facade.core.support.expressions.DataType;
import io.pact.janus.facade.internal.ContentTypes;
import io.pact.janus.facade.internal.Headers;
import io.pact.janus.facade.internal.RegexExample;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Supplier;
import java.util.regex.Pattern;
import javax.xml.transform.TransformerException;
import org.w3c.dom.Document;

/** The response to an interaction's request. */
public class PactDslResponse {
  public static final String DEFAULT_JSON_CONTENT_TYPE_REGEX = "application/json(;\\s?charset=[\\w\\-]+)?";
  private static final String CONTENT_TYPE = "Content-Type";
  private static final String APPLICATION_JSON = PactDslRequestBase.APPLICATION_JSON;
  private static final String APPLICATION_XML = PactDslRequestBase.APPLICATION_XML;

  private final ConsumerPactBuilder consumerPactBuilder;
  private final PactDslRequestWithPath request;
  private final PactDslRequestWithoutPath defaultRequestValues;
  private final PactDslResponse defaultResponseValues;
  private final PactSpecVersion version;

  private int responseStatus = 200;
  private final Map<String, List<String>> responseHeaders = new LinkedHashMap<>();
  private OptionalBody responseBody = OptionalBody.missing();
  private MatchingRules responseMatchers = new MatchingRulesImpl();
  private Generators responseGenerators = new Generators();

  public PactDslResponse(ConsumerPactBuilder consumerPactBuilder, PactDslRequestWithPath request,
      PactDslRequestWithoutPath defaultRequestValues, PactDslResponse defaultResponseValues, PactSpecVersion version) {
    this.consumerPactBuilder = consumerPactBuilder;
    this.request = request;
    this.defaultRequestValues = defaultRequestValues;
    this.defaultResponseValues = defaultResponseValues;
    this.version = version;
    if (defaultResponseValues != null) {
      responseStatus = defaultResponseValues.responseStatus;
      responseHeaders.putAll(defaultResponseValues.responseHeaders);
      responseBody = defaultResponseValues.responseBody;
      responseMatchers = ((MatchingRulesImpl) defaultResponseValues.responseMatchers).copy();
      responseGenerators = new Generators(defaultResponseValues.responseGenerators.getCategories());
    }
  }

  /** As a builder of default response values, with nothing of an interaction. */
  public PactDslResponse() {
    this(null, null, null, null, PactSpecVersion.V3);
  }

  private boolean isContentTypeHeaderNotSet() {
    return responseHeaders.keySet().stream().noneMatch(key -> key.equalsIgnoreCase(CONTENT_TYPE));
  }

  private String contentTypeHeader() {
    return responseHeaders.entrySet().stream().filter(e -> e.getKey().equalsIgnoreCase(CONTENT_TYPE))
        .map(e -> e.getValue().get(0)).findFirst().orElse("");
  }

  public PactDslResponse status(int status) {
    responseStatus = status;
    return this;
  }

  public PactDslResponse headers(Map<String, String> headers) {
    headers.forEach((key, value) -> responseHeaders.put(key, Headers.values(key, value)));
    return this;
  }

  public PactDslResponse body(String body) {
    responseBody = OptionalBody.body(body.getBytes(StandardCharsets.UTF_8));
    return this;
  }

  public PactDslResponse body(String body, String contentType) {
    responseBody = OptionalBody.body(body.getBytes(ContentTypes.charset(contentType)), contentType);
    responseHeaders.put(CONTENT_TYPE, List.of(contentType));
    return this;
  }

  public PactDslResponse body(Supplier<String> body) {
    return body(body.get());
  }

  public PactDslResponse body(Supplier<String> body, String contentType) {
    return body(body.get(), contentType);
  }

  public PactDslResponse bodyWithSingleQuotes(String body) {
    return body(QuoteUtil.convert(body));
  }

  public PactDslResponse bodyWithSingleQuotes(String body, String contentType) {
    return body(QuoteUtil.convert(body), contentType);
  }

  public PactDslResponse body(DslPart body) {
    DslPart parent = body.close();
    responseMatchers.addCategory(parent.getMatchers());
    responseGenerators.addGenerators(parent.getGenerators());
    String contentType = APPLICATION_JSON;
    if (isContentTypeHeaderNotSet()) {
      matchHeader(CONTENT_TYPE, DEFAULT_JSON_CONTENT_TYPE_REGEX, contentType);
    } else {
      contentType = contentTypeHeader();
    }
    responseBody = OptionalBody.body(parent.toString().getBytes(ContentTypes.charset(contentType)), contentType);
    return this;
  }

  public PactDslResponse body(Document body) throws TransformerException {
    String contentType = isContentTypeHeaderNotSet() ? APPLICATION_XML : contentTypeHeader();
    if (isContentTypeHeaderNotSet()) {
      responseHeaders.put(CONTENT_TYPE, List.of(contentType));
    }
    responseBody = OptionalBody.body(ConsumerPactBuilder.xmlToString(body).getBytes(ContentTypes.charset(contentType)),
        contentType);
    return this;
  }

  public PactDslResponse body(BodyBuilder builder) {
    responseMatchers.addCategory(builder.getMatchers());
    MatchingRuleCategory headerMatchers = builder.getHeaderMatchers();
    if (headerMatchers != null) {
      responseMatchers.addCategory(headerMatchers);
    }
    responseGenerators.addGenerators(builder.getGenerators());
    responseHeaders.put(CONTENT_TYPE, List.of(builder.getContentType()));
    responseBody = OptionalBody.body(builder.buildBody(), builder.getContentType());
    return this;
  }

  public PactDslResponse body(PactXmlBuilder xmlBuilder) {
    responseMatchers.addCategory(xmlBuilder.getMatchingRules());
    responseGenerators.addGenerators(xmlBuilder.getGenerators());
    if (isContentTypeHeaderNotSet()) {
      responseHeaders.put(CONTENT_TYPE, List.of(APPLICATION_XML));
      responseBody = OptionalBody.body(xmlBuilder.asBytes(), APPLICATION_XML);
    } else {
      String contentType = contentTypeHeader();
      responseBody = OptionalBody.body(xmlBuilder.asBytes(ContentTypes.charset(contentType)), contentType);
    }
    return this;
  }

  public PactDslResponse withBinaryData(byte[] example, String contentType) {
    responseBody = OptionalBody.body(example, contentType);
    responseHeaders.put(CONTENT_TYPE, List.of(contentType));
    responseMatchers.addCategory("body").addRule("$", new ContentTypeMatcher(contentType));
    return this;
  }

  public PactDslResponse bodyMatchingContentType(String contentType, String exampleContents) {
    responseBody = OptionalBody.body(exampleContents.getBytes(ContentTypes.charset(contentType)), contentType);
    responseHeaders.put(CONTENT_TYPE, List.of(contentType));
    responseMatchers.addCategory("body").addRule("$", new ContentTypeMatcher(contentType));
    return this;
  }

  /** A header matching {@code regexp}, its example drawn from the pattern the same way every run (ADR 0070 decision 3). */
  public PactDslResponse matchHeader(String header, String regexp) {
    return matchHeader(header, regexp, RegexExample.of(regexp));
  }

  public PactDslResponse matchHeader(String header, String regexp, String headerExample) {
    responseMatchers.addCategory("header").setRule(header, new RegexMatcher(regexp, null));
    responseHeaders.put(header, List.of(headerExample));
    return this;
  }

  public PactDslResponse headerFromProviderState(String name, String expression, String example) {
    responseGenerators.addGenerator(Category.HEADER, name, new ProviderStateGenerator(expression, DataType.STRING));
    responseHeaders.put(name, List.of(example));
    return this;
  }

  public PactDslResponse matchSetCookie(String cookie, String regex, String example) {
    MatchingRuleCategory header = responseMatchers.addCategory("header");
    RegexMatcher rule = new RegexMatcher(Pattern.quote(cookie + "=") + regex, null);
    if (header.getMatchingRules().containsKey("set-cookie")) {
      header.addRule("set-cookie", rule);
    } else {
      header.addRule("set-cookie", rule, RuleLogic.OR);
    }
    List<String> cookies = new ArrayList<>(responseHeaders.getOrDefault("set-cookie", List.of()));
    cookies.add(cookie + "=" + example);
    responseHeaders.put("set-cookie", cookies);
    return this;
  }

  private PactDslResponse statusMatching(String statusType, List<Integer> values, int example) {
    responseMatchers.addCategory("status").addRule(new StatusCodeMatcher(statusType, values));
    responseStatus = example;
    return this;
  }

  public PactDslResponse informationStatus() {
    return statusMatching("info", List.of(), 100);
  }

  public PactDslResponse successStatus() {
    return statusMatching("success", List.of(), 200);
  }

  public PactDslResponse redirectStatus() {
    return statusMatching("redirect", List.of(), 300);
  }

  public PactDslResponse clientErrorStatus() {
    return statusMatching("clientError", List.of(), 400);
  }

  public PactDslResponse serverErrorStatus() {
    return statusMatching("serverError", List.of(), 500);
  }

  public PactDslResponse nonErrorStatus() {
    return statusMatching("nonError", List.of(), 200);
  }

  public PactDslResponse errorStatus() {
    return statusMatching("error", List.of(), 400);
  }

  public PactDslResponse statusCodes(List<Integer> statusCodes) {
    return statusMatching("statusCodes", statusCodes, statusCodes.get(0));
  }

  /** Accepted, with no effect: a contract carries no comments (ADR 0066 decision 6). */
  public PactDslResponse comment(String comment) {
    return this;
  }

  /** Accepted, with no effect: a contract carries no pact metadata. */
  public PactDslResponse addMetadataValue(String key, Object value) {
    return this;
  }

  private void addInteraction() {
    consumerPactBuilder.getInteractions().add(new RequestResponseInteraction(request.getDescription(), request.states(),
        new RequestResponseInteraction.Part(request.requestMethod, request.getPath(), request.query, 0,
            request.requestHeaders, request.requestBody, request.requestMatchers, request.requestGenerators),
        new RequestResponseInteraction.Part(null, null, Map.of(), responseStatus, responseHeaders, responseBody,
            responseMatchers, responseGenerators)));
  }

  @SuppressWarnings("unchecked")
  public <P extends BasePact> P toPact(Class<P> pactClass) {
    addInteraction();
    Consumer consumer = new Consumer(request.consumer());
    Provider provider = new Provider(request.provider());
    if (pactClass.isAssignableFrom(V4Pact.class)) {
      return (P) new V4Pact(provider, consumer, consumerPactBuilder.getInteractions(), PactSpecVersion.V4);
    }
    if (pactClass.isAssignableFrom(RequestResponsePact.class)) {
      return (P) new RequestResponsePact(provider, consumer, consumerPactBuilder.getInteractions(), version);
    }
    throw new IllegalArgumentException(pactClass.getSimpleName() + " is not a valid Pact class");
  }

  public RequestResponsePact toPact() {
    return toPact(RequestResponsePact.class);
  }

  public PactDslRequestWithPath uponReceiving(String description) {
    addInteraction();
    return new PactDslRequestWithPath(request, description, List.of());
  }

  public PactDslWithState given(String state) {
    addInteraction();
    PactDslWithState withState = new PactDslWithState(consumerPactBuilder, request.consumer(), request.provider(),
        defaultRequestValues, defaultResponseValues, version);
    withState.state.add(new ProviderState(state));
    return withState;
  }

  public PactDslWithState given(String state, Map<String, ?> params) {
    addInteraction();
    PactDslWithState withState = new PactDslWithState(consumerPactBuilder, request.consumer(), request.provider(),
        defaultRequestValues, defaultResponseValues, version);
    withState.state.add(new ProviderState(state, params));
    return withState;
  }
}
