package io.pact.janus.facade.internal;

import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;

/** A content type's charset, as pact-jvm's {@code ContentType.asCharset} reads it. Not API. */
public final class ContentTypes {
  private ContentTypes() {}

  public static Charset charset(String contentType) {
    if (contentType != null) {
      for (String parameter : contentType.split(";")) {
        String[] pair = parameter.trim().split("=", 2);
        if (pair.length == 2 && pair[0].trim().equalsIgnoreCase("charset")) {
          try {
            return Charset.forName(pair[1].trim().replace("\"", ""));
          } catch (RuntimeException e) {
            return StandardCharsets.UTF_8;
          }
        }
      }
    }
    return StandardCharsets.UTF_8;
  }
}
