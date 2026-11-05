// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.matchingrules;

import io.pact.janus.facade.core.model.PactSpecVersion;
import io.pact.janus.facade.core.model.generators.Generator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** An array containing an item like each variant, in any order. */
public final class ArrayContainsMatcher implements MatchingRule {
  /** One variant: the index of its example, its rules and its generators. */
  public record Variant(int index, MatchingRuleCategory rules, Map<String, Generator> generators) {}

  private final List<Variant> variants;

  public ArrayContainsMatcher(List<Variant> variants) {
    this.variants = List.copyOf(variants);
  }

  public List<Variant> getVariants() {
    return variants;
  }

  @Override
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    map.put("match", "arrayContains");
    map.put("variants", variants.stream().map(variant -> {
      Map<String, Object> written = new LinkedHashMap<>();
      written.put("index", variant.index());
      written.put("rules", variant.rules().toMap(spec));
      Map<String, Object> generators = new LinkedHashMap<>();
      variant.generators().forEach((path, generator) -> generators.put(path, generator.toMap(spec)));
      written.put("generators", generators);
      return written;
    }).toList());
    return map;
  }
}
