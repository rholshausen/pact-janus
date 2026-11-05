// Ported from pact-jvm at 1928f2247 (4.7.8), consumer; ADR 0070.
package io.pact.janus.facade.consumer.dsl;

import io.pact.janus.facade.consumer.ConsumerPactBuilder;
import io.pact.janus.facade.consumer.xml.PactXmlBuilder;
import io.pact.janus.facade.core.model.OptionalBody;
import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.ProviderState;
import io.pact.janus.facade.core.model.generators.Category;
import io.pact.janus.facade.core.model.generators.DateGenerator;
import io.pact.janus.facade.core.model.generators.DateTimeGenerator;
import io.pact.janus.facade.core.model.generators.Generators;
import io.pact.janus.facade.core.model.generators.ProviderStateGenerator;
import io.pact.janus.facade.core.model.generators.TimeGenerator;
import io.pact.janus.facade.core.model.matchingrules.ContentTypeMatcher;
import io.pact.janus.facade.core.model.matchingrules.DateMatcher;
import io.pact.janus.facade.core.model.matchingrules.MatchingRules;
import io.pact.janus.facade.core.model.matchingrules.MatchingRulesImpl;
import io.pact.janus.facade.core.model.matchingrules.RegexMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimeMatcher;
import io.pact.janus.facade.core.model.matchingrules.TimestampMatcher;
import io.pact.janus.facade.core.support.expressions.DataType;
import io.pact.janus.facade.internal.ContentTypes;
import io.pact.janus.facade.internal.Dates;
import io.pact.janus.facade.internal.Headers;
import io.pact.janus.facade.internal.Multipart;
import io.pact.janus.facade.internal.RegexExample;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Supplier;
import javax.xml.transform.TransformerException;
import org.w3c.dom.Document;

/**
 * A request being built: its method, headers, query, body, rules and generators, as pact-jvm's
 * {@code PactDslRequestBase} holds them, with the setters the two request builders share.
 */
public abstract class PactDslRequestBase<SELF extends PactDslRequestBase<SELF>> {
  public static final String CONTENT_TYPE = "Content-Type";
  /** Apache HttpCore's JSON and XML types, with their charsets, which pact-jvm's builders use. */
  public static final String APPLICATION_JSON = "application/json; charset=UTF-8";
  public static final String APPLICATION_XML = "application/xml; charset=ISO-8859-1";

  protected final ConsumerPactBuilder consumerPactBuilder;
  protected final String consumerName;
  protected final String providerName;
  protected final List<ProviderState> state;
  protected final String description;
  protected final PactDslRequestWithoutPath defaultRequestValues;
  protected final PactDslResponse defaultResponseValues;
  protected final PactSpecVersion version;

  public String requestMethod = "GET";
  public Map<String, List<String>> requestHeaders = new LinkedHashMap<>();
  public Map<String, List<String>> query = new LinkedHashMap<>();
  public OptionalBody requestBody = OptionalBody.missing();
  public MatchingRules requestMatchers = new MatchingRulesImpl();
  public Generators requestGenerators = new Generators();
  protected Multipart multipart;

  protected PactDslRequestBase(ConsumerPactBuilder consumerPactBuilder, String consumerName, String providerName,
      List<ProviderState> state, String description, PactDslRequestWithoutPath defaultRequestValues,
      PactDslResponse defaultResponseValues, PactSpecVersion version) {
    this.consumerPactBuilder = consumerPactBuilder;
    this.consumerName = consumerName;
    this.providerName = providerName;
    this.state = new ArrayList<>(state);
    this.description = description;
    this.defaultRequestValues = defaultRequestValues;
    this.defaultResponseValues = defaultResponseValues;
    this.version = version;
  }

  @SuppressWarnings("unchecked")
  protected SELF self() {
    return (SELF) this;
  }

  protected void setupDefaultValues() {
    if (defaultRequestValues != null) {
      if (!defaultRequestValues.requestMethod.isEmpty()) {
        requestMethod = defaultRequestValues.requestMethod;
      }
      requestHeaders.putAll(defaultRequestValues.requestHeaders);
      query.putAll(defaultRequestValues.query);
      requestBody = defaultRequestValues.requestBody;
      requestMatchers = ((MatchingRulesImpl) defaultRequestValues.requestMatchers).copy();
      requestGenerators = new Generators(defaultRequestValues.requestGenerators.getCategories());
    }
  }

  /** Copies what {@code from} has built so far. */
  protected void copyFrom(PactDslRequestBase<?> from) {
    requestMethod = from.requestMethod;
    requestHeaders = from.requestHeaders;
    query = from.query;
    requestBody = from.requestBody;
    requestMatchers = from.requestMatchers;
    requestGenerators = from.requestGenerators;
    multipart = from.multipart;
  }

  protected boolean isContentTypeHeaderNotSet() {
    return requestHeaders.keySet().stream().noneMatch(key -> key.equalsIgnoreCase(CONTENT_TYPE));
  }

  protected String contentTypeHeader() {
    return requestHeaders.entrySet().stream().filter(e -> e.getKey().equalsIgnoreCase(CONTENT_TYPE))
        .map(e -> e.getValue().get(0)).findFirst().orElse("");
  }

  public SELF method(String method) {
    requestMethod = method;
    return self();
  }

  public SELF headers(String firstHeaderName, String firstHeaderValue, String... headerNameValuePairs) {
    if (headerNameValuePairs.length % 2 != 0) {
      throw new IllegalArgumentException("Pair key value should be provided, but there is one key without value.");
    }
    requestHeaders.put(firstHeaderName, Headers.values(firstHeaderName, firstHeaderValue));
    for (int i = 0; i < headerNameValuePairs.length; i += 2) {
      requestHeaders.put(headerNameValuePairs[i], Headers.values(headerNameValuePairs[i], headerNameValuePairs[i + 1]));
    }
    return self();
  }

  public SELF headers(Map<String, String> headers) {
    headers.forEach((key, value) -> requestHeaders.put(key, Headers.values(key, value)));
    return self();
  }

  public SELF query(String query) {
    this.query = Headers.queryStringToMap(query, false);
    return self();
  }

  public SELF encodedQuery(String query) {
    this.query = Headers.queryStringToMap(query, true);
    return self();
  }

  public SELF body(String body) {
    requestBody = OptionalBody.body(body.getBytes(StandardCharsets.UTF_8));
    return self();
  }

  public SELF body(String body, String contentType) {
    requestBody = OptionalBody.body(body.getBytes(ContentTypes.charset(contentType)), contentType);
    requestHeaders.put(CONTENT_TYPE, List.of(contentType));
    return self();
  }

  public SELF body(Supplier<String> body) {
    return body(body.get());
  }

  public SELF body(Supplier<String> body, String contentType) {
    return body(body.get(), contentType);
  }

  public SELF bodyWithSingleQuotes(String body) {
    return body(QuoteUtil.convert(body));
  }

  public SELF bodyWithSingleQuotes(String body, String contentType) {
    return body(QuoteUtil.convert(body), contentType);
  }

  public SELF body(DslPart body) {
    DslPart parent = body.close();
    requestMatchers.addCategory(parent.getMatchers());
    requestGenerators.addGenerators(parent.getGenerators());
    String contentType = APPLICATION_JSON;
    if (isContentTypeHeaderNotSet()) {
      requestHeaders.put(CONTENT_TYPE, List.of(contentType));
    } else {
      contentType = contentTypeHeader();
    }
    requestBody = OptionalBody.body(parent.toString().getBytes(ContentTypes.charset(contentType)), contentType);
    return self();
  }

  public SELF body(Document body) throws TransformerException {
    String contentType = isContentTypeHeaderNotSet() ? APPLICATION_XML : contentTypeHeader();
    if (isContentTypeHeaderNotSet()) {
      requestHeaders.put(CONTENT_TYPE, List.of(contentType));
    }
    requestBody = OptionalBody.body(ConsumerPactBuilder.xmlToString(body).getBytes(ContentTypes.charset(contentType)),
        contentType);
    return self();
  }

  public SELF body(PactXmlBuilder xmlBuilder) {
    requestMatchers.addCategory(xmlBuilder.getMatchingRules());
    requestGenerators.addGenerators(xmlBuilder.getGenerators());
    if (isContentTypeHeaderNotSet()) {
      requestHeaders.put(CONTENT_TYPE, List.of(APPLICATION_XML));
      requestBody = OptionalBody.body(xmlBuilder.asBytes(), APPLICATION_XML);
    } else {
      String contentType = contentTypeHeader();
      requestBody = OptionalBody.body(xmlBuilder.asBytes(ContentTypes.charset(contentType)), contentType);
    }
    return self();
  }

  public SELF body(BodyBuilder builder) {
    requestMatchers.addCategory(builder.getMatchers());
    if (builder.getHeaderMatchers() != null) {
      requestMatchers.addCategory(builder.getHeaderMatchers());
    }
    requestGenerators.addGenerators(builder.getGenerators());
    requestHeaders.put(CONTENT_TYPE, List.of(builder.getContentType()));
    requestBody = OptionalBody.body(builder.buildBody(), builder.getContentType());
    return self();
  }

  public SELF bodyMatchingContentType(String contentType, String exampleContents) {
    requestBody = OptionalBody.body(exampleContents.getBytes(ContentTypes.charset(contentType)), contentType);
    requestHeaders.put(CONTENT_TYPE, List.of(contentType));
    requestMatchers.addCategory("body").addRule("$", new ContentTypeMatcher(contentType));
    return self();
  }

  public SELF withBinaryData(byte[] example, String contentType) {
    requestBody = OptionalBody.body(example, contentType);
    requestHeaders.put(CONTENT_TYPE, List.of(contentType));
    requestMatchers.addCategory("body").addRule("$", new ContentTypeMatcher(contentType));
    return self();
  }

  /** A file as a part of a {@code multipart/form-data} body. */
  public SELF withFileUpload(String partName, String fileName, String fileContentType, byte[] data) {
    if (multipart == null) {
      multipart = new Multipart();
    }
    multipart.add(partName, fileName, fileContentType == null || fileContentType.isEmpty() ? "text/plain" : fileContentType, data);
    requestBody = multipart.body();
    if (!requestMatchers.addCategory("header").getMatchingRules().containsKey(CONTENT_TYPE)) {
      requestMatchers.addCategory("header").addRule(CONTENT_TYPE,
          new RegexMatcher(Multipart.HEADER_REGEX, multipart.contentType()));
    }
    if (!requestHeaders.containsKey(CONTENT_TYPE)) {
      requestHeaders.put(CONTENT_TYPE, List.of(multipart.contentType()));
    }
    return self();
  }

  /** A header matching {@code regex}, its example drawn from the pattern the same way every run (ADR 0070 decision 3). */
  public SELF matchHeader(String header, String regex) {
    return matchHeader(header, regex, RegexExample.of(regex));
  }

  public SELF matchHeader(String header, String regex, String headerExample) {
    if (!headerExample.matches(regex)) {
      throw new InvalidMatcherException("Example \"" + headerExample + "\" does not match regular expression \"" + regex + "\"");
    }
    requestMatchers.addCategory("header").setRule(header, new RegexMatcher(regex, null));
    requestHeaders.put(header, List.of(headerExample));
    return self();
  }

  /** A query parameter matching {@code regex}, its example drawn from the pattern the same way every run (ADR 0070 decision 3). */
  public SELF matchQuery(String parameter, String regex) {
    return matchQuery(parameter, regex, RegexExample.of(regex));
  }

  public SELF matchQuery(String parameter, String regex, String example) {
    if (!example.matches(regex)) {
      throw new InvalidMatcherException("Example \"" + example + "\" does not match regular expression \"" + regex + "\"");
    }
    requestMatchers.addCategory("query").addRule(parameter, new RegexMatcher(regex, null));
    query.put(parameter, List.of(example));
    return self();
  }

  public SELF matchQuery(String parameter, String regex, List<String> example) {
    for (String e : example) {
      if (!e.matches(regex)) {
        throw new InvalidMatcherException("Example \"" + e + "\" does not match regular expression \"" + regex + "\"");
      }
    }
    requestMatchers.addCategory("query").addRule(parameter, new RegexMatcher(regex, null));
    query.put(parameter, new ArrayList<>(example));
    return self();
  }

  public SELF headerFromProviderState(String name, String expression, String example) {
    requestGenerators.addGenerator(Category.HEADER, name, new ProviderStateGenerator(expression, DataType.STRING));
    requestHeaders.put(name, List.of(example));
    return self();
  }

  public SELF queryParameterFromProviderState(String name, String expression, String example) {
    requestGenerators.addGenerator(Category.QUERY, name, new ProviderStateGenerator(expression, DataType.STRING));
    query.put(name, List.of(example));
    return self();
  }

  private SELF queryMatching(String field, String pattern, String example,
      io.pact.janus.facade.core.model.matchingrules.MatchingRule rule,
      io.pact.janus.facade.core.model.generators.Generator generator) {
    requestMatchers.addCategory("query").addRule(field, rule);
    if (example != null && !example.isEmpty()) {
      query.put(field, List.of(example));
    } else {
      requestGenerators.addGenerator(Category.QUERY, field, generator);
      query.put(field, List.of(Dates.local2000(pattern)));
    }
    return self();
  }

  public SELF queryMatchingDate(String field, String pattern, String example) {
    return queryMatching(field, pattern, example, new DateMatcher(pattern), new DateGenerator(pattern, null));
  }

  public SELF queryMatchingDate(String field, String pattern) {
    return queryMatchingDate(field, pattern, null);
  }

  public SELF queryMatchingTime(String field, String pattern, String example) {
    return queryMatching(field, pattern, example, new TimeMatcher(pattern), new TimeGenerator(pattern, null));
  }

  public SELF queryMatchingTime(String field, String pattern) {
    return queryMatchingTime(field, pattern, null);
  }

  public SELF queryMatchingDatetime(String field, String pattern, String example) {
    return queryMatching(field, pattern, example, new TimestampMatcher(pattern), new DateTimeGenerator(pattern, null));
  }

  public SELF queryMatchingDatetime(String field, String pattern) {
    return queryMatchingDatetime(field, pattern, null);
  }

  public SELF queryMatchingISODate(String field) {
    return queryMatchingISODate(field, null);
  }

  public SELF queryMatchingISODate(String field, String example) {
    return queryMatchingDate(field, "yyyy-MM-dd", example);
  }

  public SELF queryMatchingISOTime(String field, String example) {
    return queryMatchingTime(field, "HH:mm:ss", example);
  }

  public SELF queryMatchingTime(String field) {
    return queryMatchingISOTime(field, null);
  }

  public SELF queryMatchingISODatetime(String field) {
    return queryMatchingISODatetime(field, null);
  }

  public SELF queryMatchingISODatetime(String field, String example) {
    return queryMatchingDatetime(field, "yyyy-MM-dd'T'HH:mm:ssXXX", example);
  }

  /** Accepted, with no effect: a contract carries no comments (ADR 0066 decision 6). */
  public SELF comment(String comment) {
    return self();
  }

  /** Accepted, with no effect: a contract carries no pact metadata. */
  public SELF addMetadataValue(String key, Object value) {
    return self();
  }
}
