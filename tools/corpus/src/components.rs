//! The component conformance corpus (component-interfaces spec §9.4, staged plan S5.10, ADR 0105):
//! every case under `corpora/components/<component>/`, run through the built-in component's native
//! binding and the serialising in-process one, which must answer every call as the case expects and
//! as each other.
//!
//! The runner is the kernel's ([`conformance::run`]), the one `janus component check` runs an
//! out-of-tree component's cases with. What is here is which components are built in, and where their
//! cases are.

use pact_janus_kernel::component::conformance::{self, Binding, Case, Report};
use pact_janus_kernel::component::{ContentComponent, MatcherComponent};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A component compiled into the native embeddings, under the name it is declared in-tree by.
pub struct BuiltIn {
  pub name: &'static str,
  pub content: Option<Arc<dyn ContentComponent>>,
  pub matcher: Option<Arc<dyn MatcherComponent>>,
}

/// Every built-in component with an interface the corpus can call: the content components
/// `janus-engine` registers, and the HTTP component's matcher.
pub fn built_ins() -> Vec<BuiltIn> {
  fn content(name: &'static str, component: impl ContentComponent + 'static) -> BuiltIn {
    BuiltIn {
      name,
      content: Some(Arc::new(component)),
      matcher: None,
    }
  }
  vec![
    content("json", pact_janus_component_json::JsonContent::new()),
    content("text", pact_janus_component_text::TextContent::new()),
    content("form", pact_janus_component_form::FormContent::new()),
    content("xml", pact_janus_component_xml::XmlContent::new()),
    content(
      "multipart",
      pact_janus_component_multipart::MultipartContent::new(),
    ),
    BuiltIn {
      name: pact_janus_component_http::status::NAME,
      content: None,
      matcher: Some(Arc::new(pact_janus_component_http::status::HttpMatcher::new())),
    },
  ]
}

impl BuiltIn {
  /// The bindings it is reached by in tests: natively, and behind frames.
  pub fn bindings(&self) -> Vec<Binding> {
    vec![
      Binding::native(self.content.clone(), self.matcher.clone()),
      Binding::serialising(self.name, self.content.clone(), self.matcher.clone()),
    ]
  }
}

/// Every case file under `root` (`corpora/components/`), in path order.
pub fn discover(root: &Path) -> Vec<PathBuf> {
  let mut cases = Vec::new();
  let mut dirs = vec![root.to_path_buf()];
  while let Some(dir) = dirs.pop() {
    let Ok(entries) = std::fs::read_dir(&dir) else {
      continue;
    };
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_dir() {
        dirs.push(path);
      } else if path.extension().is_some_and(|extension| extension == "json") {
        cases.push(path);
      }
    }
  }
  cases.sort();
  cases
}

/// Run one case file through the built-in it names. A case sits in the directory named for its
/// component, so a reader finds a component's cases in one place.
pub fn run_file(path: &Path, built_ins: &[BuiltIn]) -> Result<Report, String> {
  let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
  let case = Case::read(&text)?;
  let directory = path
    .parent()
    .and_then(Path::file_name)
    .and_then(|name| name.to_str())
    .unwrap_or_default();
  if directory != case.component {
    return Err(format!(
      "it is a case of component '{}', and sits in '{directory}/'",
      case.component
    ));
  }
  let built_in = built_ins
    .iter()
    .find(|built_in| built_in.name == case.component)
    .ok_or_else(|| format!("no built-in component is named '{}'", case.component))?;
  Ok(conformance::run(&case, &built_in.bindings()))
}
