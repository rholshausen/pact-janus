// Ported from pact-jvm at 1928f2247 (4.7.8), core/support/src/main/kotlin/.../support/regex/
// (RegexNode.kt, RegexParser.kt, RegexStringGenerator.kt) and Random.generateRandomString; ADR 0070.
package io.pact.janus.facade.internal;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Random;

/**
 * An example of a regular expression, where pact-jvm's DSL is given none: drawn as pact-jvm draws one,
 * but from a generator seeded by the pattern, so every run records the same contract (ADR 0070
 * decision 3). Not API.
 */
public final class RegexExample {
  private static final int MAX_UNBOUNDED = 5;
  private static final List<Character> PRINTABLE_ASCII = range(' ', '~');

  private sealed interface Node permits Literal, CharClass, AnyChar, Sequence, Alternation, Repeat, Group {}

  private record Literal(char c) implements Node {}

  private record CharClass(List<char[]> ranges, List<Character> chars, boolean negated) implements Node {
    List<Character> toCharSet() {
      LinkedHashSet<Character> included = new LinkedHashSet<>();
      for (char[] range : ranges) {
        included.addAll(range(range[0], range[1]));
      }
      included.addAll(chars);
      if (negated) {
        return PRINTABLE_ASCII.stream().filter(c -> !included.contains(c)).toList();
      }
      return new ArrayList<>(included);
    }
  }

  private record AnyChar() implements Node {}

  private record Sequence(List<Node> nodes) implements Node {}

  private record Alternation(List<Node> alternatives) implements Node {}

  private record Repeat(Node node, int min, int max) implements Node {}

  private record Group(Node node) implements Node {}

  private final String pattern;
  private int pos;
  private final Random random;

  private RegexExample(String pattern, Random random) {
    this.pattern = pattern;
    this.random = random;
  }

  /** An example of {@code regex}, the same one every time. */
  public static String of(String regex) {
    String cleaned = regex.endsWith("$") && !regex.endsWith("\\$")
        ? stripEnd(stripStart(regex, '^'), '$')
        : stripStart(regex, '^');
    RegexExample generator = new RegexExample(cleaned, new Random(seed(regex)));
    StringBuilder out = new StringBuilder();
    generator.append(out, generator.parseAlternation());
    return out.toString();
  }

  private static long seed(String regex) {
    try {
      byte[] digest = MessageDigest.getInstance("SHA-256").digest(regex.getBytes(StandardCharsets.UTF_8));
      long seed = 0;
      for (int i = 0; i < 8; i++) {
        seed = (seed << 8) | (digest[i] & 0xff);
      }
      return seed;
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException(e);
    }
  }

  private static String stripStart(String value, char c) {
    int start = 0;
    while (start < value.length() && value.charAt(start) == c) {
      start++;
    }
    return value.substring(start);
  }

  private static String stripEnd(String value, char c) {
    int end = value.length();
    while (end > 0 && value.charAt(end - 1) == c) {
      end--;
    }
    return value.substring(0, end);
  }

  private static List<Character> range(char from, char to) {
    List<Character> chars = new ArrayList<>();
    for (char c = from; c <= to; c++) {
      chars.add(c);
    }
    return chars;
  }

  // ----------------------------------------------------------------------------------------------
  // Generating

  private void append(StringBuilder out, Node node) {
    if (node instanceof Literal literal) {
      out.append(literal.c());
    } else if (node instanceof CharClass charClass) {
      List<Character> chars = charClass.toCharSet();
      if (!chars.isEmpty()) {
        out.append(chars.get(random.nextInt(chars.size())));
      }
    } else if (node instanceof AnyChar) {
      out.append(PRINTABLE_ASCII.get(random.nextInt(PRINTABLE_ASCII.size())));
    } else if (node instanceof Sequence sequence) {
      sequence.nodes().forEach(n -> append(out, n));
    } else if (node instanceof Alternation alternation) {
      append(out, alternation.alternatives().get(random.nextInt(alternation.alternatives().size())));
    } else if (node instanceof Repeat repeat) {
      int count = repetitions(repeat);
      for (int i = 0; i < count; i++) {
        append(out, repeat.node());
      }
    } else if (node instanceof Group group) {
      append(out, group.node());
    }
  }

  private int repetitions(Repeat node) {
    if (node.min() == node.max()) {
      return node.min();
    }
    int effectiveMax = node.max() == Integer.MAX_VALUE
        ? Math.max(node.min(), MAX_UNBOUNDED)
        : Math.min(node.max(), node.min() + MAX_UNBOUNDED);
    return node.min() >= effectiveMax ? node.min() : node.min() + random.nextInt(effectiveMax - node.min() + 1);
  }

  // ----------------------------------------------------------------------------------------------
  // Parsing

  private boolean at(char c) {
    return pos < pattern.length() && pattern.charAt(pos) == c;
  }

  private Node parseAlternation() {
    List<Node> alternatives = new ArrayList<>(List.of(parseSequence()));
    while (at('|')) {
      pos++;
      alternatives.add(parseSequence());
    }
    return alternatives.size() == 1 ? alternatives.get(0) : new Alternation(alternatives);
  }

  private Node parseSequence() {
    List<Node> nodes = new ArrayList<>();
    while (pos < pattern.length() && !at(')') && !at('|')) {
      nodes.add(parseQuantifier(parseAtom()));
    }
    return nodes.size() == 1 ? nodes.get(0) : new Sequence(nodes);
  }

  private Node parseAtom() {
    if (pos >= pattern.length()) {
      return new Sequence(List.of());
    }
    char c = pattern.charAt(pos);
    switch (c) {
      case '(':
        return parseGroup();
      case '[':
        return parseCharClass();
      case '.':
        pos++;
        return new AnyChar();
      case '\\':
        return parseEscape();
      case '^':
      case '$':
        pos++;
        return new Sequence(List.of());
      default:
        pos++;
        return new Literal(c);
    }
  }

  private Node parseGroup() {
    pos++;
    boolean ignore = false;
    if (at('?')) {
      pos++;
      if (at(':') || at('>')) {
        pos++;
      } else if (at('=') || at('!')) {
        pos++;
        ignore = true;
      } else if (at('<')) {
        pos++;
        if (at('=') || at('!')) {
          pos++;
          ignore = true;
        } else {
          while (pos < pattern.length() && !at('>')) {
            pos++;
          }
          if (pos < pattern.length()) {
            pos++;
          }
        }
      } else {
        while (pos < pattern.length() && !at(':') && !at(')')) {
          pos++;
        }
        if (at(':')) {
          pos++;
        }
      }
    }
    Node inner = parseAlternation();
    if (at(')')) {
      pos++;
    }
    return ignore ? new Sequence(List.of()) : new Group(inner);
  }

  private record Members(List<char[]> ranges, List<Character> chars) {}

  private static final Members WORD = members(List.of(new char[] {'a', 'z'}, new char[] {'A', 'Z'}, new char[] {'0', '9'}), List.of('_'));
  private static final Members NOT_WORD = members(List.of(new char[] {' ', '/'}, new char[] {':', '@'},
      new char[] {'[', '^'}, new char[] {'{', '~'}), List.of('`'));
  private static final Members DIGIT = members(List.of(new char[] {'0', '9'}), List.of());
  private static final Members NOT_DIGIT = members(List.of(new char[] {' ', '/'}, new char[] {':', '~'}), List.of());
  private static final Members SPACE = members(List.of(), List.of(' ', '\t', '\n', '\u000B', '\r', '\u000C'));
  private static final Members NOT_SPACE = members(List.of(new char[] {'!', '~'}), List.of());
  private static final Members ALPHA = members(List.of(new char[] {'a', 'z'}, new char[] {'A', 'Z'}), List.of());
  private static final Members ALNUM = members(List.of(new char[] {'a', 'z'}, new char[] {'A', 'Z'}, new char[] {'0', '9'}), List.of());
  private static final Members PUNCT = members(List.of(new char[] {'!', '/'}, new char[] {':', '@'},
      new char[] {'[', '`'}, new char[] {'{', '~'}), List.of());
  private static final Members XDIGIT = members(List.of(new char[] {'0', '9'}, new char[] {'a', 'f'}, new char[] {'A', 'F'}), List.of());
  private static final Members PRINT = members(List.of(new char[] {' ', '~'}), List.of());

  private static Members members(List<char[]> ranges, List<Character> chars) {
    return new Members(ranges, chars);
  }

  private Node parseCharClass() {
    pos++;
    boolean negated = at('^');
    if (negated) {
      pos++;
    }
    List<char[]> ranges = new ArrayList<>();
    List<Character> chars = new ArrayList<>();
    if (at(']')) {
      chars.add(']');
      pos++;
    }
    while (pos < pattern.length() && !at(']')) {
      if (at('\\')) {
        pos++;
        Members members = parseClassEscape();
        ranges.addAll(members.ranges());
        chars.addAll(members.chars());
      } else if (at('[') && pos + 1 < pattern.length() && pattern.charAt(pos + 1) == ':') {
        pos += 2;
        StringBuilder name = new StringBuilder();
        while (pos < pattern.length() && !at(':')) {
          name.append(pattern.charAt(pos++));
        }
        if (pos + 1 < pattern.length() && at(':') && pattern.charAt(pos + 1) == ']') {
          pos += 2;
        }
        Members members = posix(name.toString());
        ranges.addAll(members.ranges());
        chars.addAll(members.chars());
      } else {
        char start = pattern.charAt(pos++);
        if (pos + 1 < pattern.length() && at('-') && pattern.charAt(pos + 1) != ']') {
          pos++;
          char end;
          if (at('\\')) {
            pos++;
            end = parseSingleEscape();
          } else if (pos < pattern.length()) {
            end = pattern.charAt(pos++);
          } else {
            end = start;
          }
          if (start <= end) {
            ranges.add(new char[] {start, end});
          } else {
            chars.add(start);
            chars.add('-');
            chars.add(end);
          }
        } else {
          chars.add(start);
        }
      }
    }
    if (pos < pattern.length()) {
      pos++;
    }
    return new CharClass(ranges, chars, negated);
  }

  private Members parseClassEscape() {
    if (pos >= pattern.length()) {
      return members(List.of(), List.of());
    }
    char c = pattern.charAt(pos++);
    return switch (c) {
      case 'w' -> WORD;
      case 'W' -> NOT_WORD;
      case 'd' -> DIGIT;
      case 'D' -> NOT_DIGIT;
      case 's' -> SPACE;
      case 'S' -> NOT_SPACE;
      case 'p', 'P' -> unicodeProperty();
      case 'n' -> members(List.of(), List.of('\n'));
      case 't' -> members(List.of(), List.of('\t'));
      case 'r' -> members(List.of(), List.of('\r'));
      case 'f' -> members(List.of(), List.of('\u000C'));
      case '0' -> members(List.of(), List.of('\u0000'));
      case 'x' -> members(List.of(), List.of(hex(2)));
      case 'u' -> members(List.of(), List.of(hex(4)));
      default -> members(List.of(), List.of(c));
    };
  }

  private char parseSingleEscape() {
    if (pos >= pattern.length()) {
      return '\\';
    }
    char c = pattern.charAt(pos++);
    return switch (c) {
      case 'n' -> '\n';
      case 't' -> '\t';
      case 'r' -> '\r';
      case 'f' -> '\u000C';
      case '0' -> '\u0000';
      case 'x' -> hex(2);
      case 'u' -> hex(4);
      default -> c;
    };
  }

  private Node parseEscape() {
    pos++;
    if (pos >= pattern.length()) {
      throw new IllegalArgumentException("Trailing backslash in regex pattern");
    }
    char c = pattern.charAt(pos++);
    return switch (c) {
      case 'w' -> new CharClass(WORD.ranges(), WORD.chars(), false);
      case 'W' -> new CharClass(NOT_WORD.ranges(), NOT_WORD.chars(), false);
      case 'd' -> new CharClass(DIGIT.ranges(), DIGIT.chars(), false);
      case 'D' -> new CharClass(NOT_DIGIT.ranges(), NOT_DIGIT.chars(), false);
      case 's' -> new CharClass(SPACE.ranges(), SPACE.chars(), false);
      case 'S' -> new CharClass(NOT_SPACE.ranges(), NOT_SPACE.chars(), false);
      case 'n' -> new Literal('\n');
      case 't' -> new Literal('\t');
      case 'r' -> new Literal('\r');
      case 'f' -> new Literal('\u000C');
      case '0' -> new Literal('\u0000');
      case 'b', 'B' -> new Sequence(List.of());
      case 'p' -> {
        Members members = unicodeProperty();
        yield new CharClass(members.ranges(), members.chars(), false);
      }
      case 'P' -> {
        Members members = unicodeProperty();
        yield new CharClass(members.ranges(), members.chars(), true);
      }
      case 'x' -> new Literal(hex(2));
      case 'u' -> new Literal(hex(4));
      default -> c >= '1' && c <= '9' ? new Sequence(List.of()) : new Literal(c);
    };
  }

  private char hex(int digits) {
    String hex;
    if (at('{')) {
      pos++;
      StringBuilder text = new StringBuilder();
      while (pos < pattern.length() && !at('}')) {
        text.append(pattern.charAt(pos++));
      }
      if (pos < pattern.length()) {
        pos++;
      }
      hex = text.toString();
    } else {
      int end = Math.min(pos + digits, pattern.length());
      hex = pattern.substring(pos, end);
      pos = end;
    }
    return (char) Integer.parseInt(hex, 16);
  }

  private Members unicodeProperty() {
    String name;
    if (at('{')) {
      pos++;
      StringBuilder text = new StringBuilder();
      while (pos < pattern.length() && !at('}')) {
        text.append(pattern.charAt(pos++));
      }
      if (pos < pattern.length()) {
        pos++;
      }
      name = text.toString();
    } else if (pos < pattern.length()) {
      name = String.valueOf(pattern.charAt(pos++));
    } else {
      name = "L";
    }
    return switch (name.toLowerCase(java.util.Locale.ROOT)) {
      case "l", "letter", "alpha", "is_alpha", "javaletterordigit" -> ALPHA;
      case "lu", "uppercase_letter", "upper", "is_upper", "javauppercase" -> members(List.of(new char[] {'A', 'Z'}), List.of());
      case "ll", "lowercase_letter", "lower", "is_lower", "javalowercase" -> members(List.of(new char[] {'a', 'z'}), List.of());
      case "n", "nd", "digit", "is_digit", "javadigit" -> DIGIT;
      case "alnum", "alphanumeric" -> ALNUM;
      case "word" -> WORD;
      case "s", "z", "zs", "space", "whitespace", "javawhitespace" -> SPACE;
      case "p", "po", "punct" -> PUNCT;
      case "ascii", "is_ascii", "print", "graph" -> PRINT;
      case "xdigit", "hex_digit" -> XDIGIT;
      default -> ALNUM;
    };
  }

  private static Members posix(String name) {
    return switch (name.toLowerCase(java.util.Locale.ROOT)) {
      case "alpha" -> ALPHA;
      case "digit" -> DIGIT;
      case "alnum" -> ALNUM;
      case "space" -> SPACE;
      case "blank" -> members(List.of(), List.of(' ', '\t'));
      case "upper" -> members(List.of(new char[] {'A', 'Z'}), List.of());
      case "lower" -> members(List.of(new char[] {'a', 'z'}), List.of());
      case "punct" -> PUNCT;
      case "xdigit" -> XDIGIT;
      case "print", "graph" -> PRINT;
      default -> ALPHA;
    };
  }

  private Node parseQuantifier(Node node) {
    if (pos >= pattern.length()) {
      return node;
    }
    Node quantified;
    char c = pattern.charAt(pos);
    if (c == '*') {
      pos++;
      quantified = new Repeat(node, 0, Integer.MAX_VALUE);
    } else if (c == '+') {
      pos++;
      quantified = new Repeat(node, 1, Integer.MAX_VALUE);
    } else if (c == '?') {
      pos++;
      quantified = new Repeat(node, 0, 1);
    } else if (c == '{') {
      int saved = pos;
      pos++;
      if (pos >= pattern.length() || !Character.isDigit(pattern.charAt(pos))) {
        pos = saved;
        return node;
      }
      int min = number();
      if (at('}')) {
        pos++;
        quantified = new Repeat(node, min, min);
      } else if (at(',')) {
        pos++;
        if (at('}')) {
          pos++;
          quantified = new Repeat(node, min, Integer.MAX_VALUE);
        } else if (pos < pattern.length() && Character.isDigit(pattern.charAt(pos))) {
          int max = number();
          if (at('}') && max >= min) {
            pos++;
            quantified = new Repeat(node, min, max);
          } else {
            pos = saved;
            return node;
          }
        } else {
          pos = saved;
          return node;
        }
      } else {
        pos = saved;
        return node;
      }
    } else {
      return node;
    }
    if (at('?') || at('+')) {
      pos++;
    }
    return quantified;
  }

  private int number() {
    int n = 0;
    while (pos < pattern.length() && Character.isDigit(pattern.charAt(pos))) {
      n = n * 10 + (pattern.charAt(pos++) - '0');
    }
    return n;
  }
}
