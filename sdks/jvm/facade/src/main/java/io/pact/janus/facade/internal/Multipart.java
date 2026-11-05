package io.pact.janus.facade.internal;

import io.pact.janus.facade.core.model.OptionalBody;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;

/**
 * A {@code multipart/form-data} body of file parts, as pact-jvm's {@code withFileUpload} builds one,
 * but with a boundary derived from the parts rather than drawn at random, so every run records the
 * same bytes (ADR 0066 decision 7). Not API.
 */
public final class Multipart {
  /** The rule pact-jvm puts on the request's {@code Content-Type}. */
  public static final String HEADER_REGEX = "multipart/form-data;(\\s*charset=[^;]*;)?\\s*boundary=.*";

  private record Part(String name, String fileName, String contentType, byte[] data) {}

  private final List<Part> parts = new ArrayList<>();

  public void add(String name, String fileName, String contentType, byte[] data) {
    parts.add(new Part(name, fileName, contentType, data));
  }

  public String boundary() {
    try {
      MessageDigest digest = MessageDigest.getInstance("SHA-256");
      for (Part part : parts) {
        digest.update(part.name().getBytes(StandardCharsets.UTF_8));
        digest.update(part.fileName().getBytes(StandardCharsets.UTF_8));
        digest.update(part.data());
      }
      return HexFormat.of().formatHex(digest.digest()).substring(0, 32);
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException(e);
    }
  }

  public String contentType() {
    return "multipart/form-data; boundary=" + boundary();
  }

  public OptionalBody body() {
    String boundary = boundary();
    ByteArrayOutputStream out = new ByteArrayOutputStream();
    for (Part part : parts) {
      String head = "--" + boundary + "\r\n"
          + "Content-Disposition: form-data; name=\"" + part.name() + "\"; filename=\"" + part.fileName() + "\"\r\n"
          + "Content-Type: " + part.contentType() + "\r\n\r\n";
      out.writeBytes(head.getBytes(StandardCharsets.UTF_8));
      out.writeBytes(part.data());
      out.writeBytes("\r\n".getBytes(StandardCharsets.UTF_8));
    }
    out.writeBytes(("--" + boundary + "--\r\n").getBytes(StandardCharsets.UTF_8));
    return OptionalBody.body(out.toByteArray(), contentType());
  }
}
