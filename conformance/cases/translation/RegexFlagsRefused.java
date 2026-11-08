// translation/regex-flags-refused, on the JVM: a Pattern compiled with a flag its text does not write
// carries the flag out of band. A flag written inline, as "(?i)", is the text's own.
import io.pact.janus.sdk.Shapes;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

class RegexFlagsRefused {
  @Test
  void patternWithFlags() {
    Shapes.regex(Pattern.compile("ord-\\d+", Pattern.CASE_INSENSITIVE), "ORD-42");
  }

  @Test
  void patternWithoutFlags() {
    Shapes.regex(Pattern.compile("(?i)ord-\\d+"), "ORD-42");
  }
}
