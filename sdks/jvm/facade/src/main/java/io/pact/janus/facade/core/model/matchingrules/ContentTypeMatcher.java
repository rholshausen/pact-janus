// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** Octets of a content type. */
public final class ContentTypeMatcher implements MatchingRule {
  private final String contentType;

  public ContentTypeMatcher(String contentType) {
    this.contentType = contentType;
  }

  public String getContentType() {
    return contentType;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "contentType");
    map.put("value", contentType);
    return map;
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof ContentTypeMatcher that && java.util.Objects.equals(contentType, that.contentType);
  }

  @Override
  public int hashCode() {
    return java.util.Objects.hash(contentType);
  }

  @Override
  public String toString() {
    return "ContentTypeMatcher(" + "contentType=" + contentType + ")";
  }
}
