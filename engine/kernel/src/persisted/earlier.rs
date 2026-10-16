// Earlier spellings of each persisted identifier in `../persisted.rs` (staged plan S0.5, ADR 0031):
// read, never written. Written by the rename (.github/scripts/rename.sh), which appends each
// identifier's current spelling here before renaming it, and never renames this file. An entry is
// removed by hand, one major version after the rename that added it (Documentation/codename.md).

pub const CONTRACT_FORMAT: &[&str] = &[];
pub const PROVIDER_SHAPE_FORMAT: &[&str] = &[];
pub const COMPONENT_ARTIFACT_TYPE: &[&str] = &[];
pub const COMPONENT_CONFIG_MEDIA_TYPE: &[&str] = &[];
pub const COMPONENT_INTERFACE: &[&str] = &[];
pub const HOOK_SCRIPT_GLOBAL: &[&str] = &[];
pub const HOOK_BODY_FORMAT: &[&str] = &[];
