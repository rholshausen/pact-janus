package io.pact.janus.facade.core.support;

import java.util.List;
import java.util.Map;

/** What {@link SimpleHttp} answers, as pact-jvm's own does. Test-only. */
public record Response(int statusCode, Map<String, List<String>> headers, String body) {
  public int getStatusCode() {
    return statusCode;
  }

  public Map<String, List<String>> getHeaders() {
    return headers;
  }

  public String getBody() {
    return body;
  }
}
