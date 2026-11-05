// Ported from pact-jvm at 1928f2247 (4.7.8), core/model; ADR 0070.
package io.pact.janus.facade.core.model.generators;

import io.pact.janus.facade.core.model.PactSpecVersion;
import java.util.LinkedHashMap;
import java.util.Map;

/** A part's generators, by category and key. */
public final class Generators {
  private final Map<Category, Map<String, Generator>> categories = new LinkedHashMap<>();

  public Generators() {}

  public Generators(Map<Category, Map<String, Generator>> categories) {
    categories.forEach((category, generators) -> this.categories.put(category, new LinkedHashMap<>(generators)));
  }

  public Map<Category, Map<String, Generator>> getCategories() {
    return categories;
  }

  public Generators addGenerator(Category category, String key, Generator generator) {
    categories.computeIfAbsent(category, c -> new LinkedHashMap<>()).put(key, generator);
    return this;
  }

  public Generators addGenerator(Category category, Generator generator) {
    return addGenerator(category, "", generator);
  }

  public Generators addGenerators(Generators other) {
    other.categories.forEach((category, generators) ->
        categories.computeIfAbsent(category, c -> new LinkedHashMap<>()).putAll(generators));
    return this;
  }

  /** Re-keys every generator under {@code prefix}, in place of a leading {@code $}, unless it has it. */
  public void applyRootPrefix(String prefix) {
    categories.replaceAll((category, generators) -> {
      Map<String, Generator> rekeyed = new LinkedHashMap<>();
      generators.forEach((key, generator) -> {
        String path = key.startsWith(prefix) ? key : key.startsWith("$") ? prefix + key.substring(1) : prefix + key;
        rekeyed.put(path, generator);
      });
      return rekeyed;
    });
  }

  /** A copy with every generator re-keyed under {@code prefix}. */
  public Generators copyWithUpdatedMatcherRootPrefix(String prefix) {
    Generators copy = new Generators(categories);
    copy.applyRootPrefix(prefix);
    return copy;
  }

  public boolean isEmpty() {
    return categories.values().stream().allMatch(Map::isEmpty);
  }

  public boolean isNotEmpty() {
    return !isEmpty();
  }

  /** As a pact writes them: by category, and the path, status and method categories as one generator. */
  public Map<String, Object> toMap(PactSpecVersion spec) {
    Map<String, Object> map = new LinkedHashMap<>();
    categories.forEach((category, generators) -> {
      if (generators.isEmpty()) {
        return;
      }
      if ((category == Category.PATH || category == Category.STATUS || category == Category.METHOD)
          && generators.containsKey("")) {
        map.put(category.key(), generators.get("").toMap(spec));
      } else {
        Map<String, Object> written = new LinkedHashMap<>();
        generators.forEach((key, generator) -> written.put(key, generator.toMap(spec)));
        map.put(category.key(), written);
      }
    });
    return map;
  }
}
