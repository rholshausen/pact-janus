use std::path::PathBuf;

fn main() -> std::io::Result<()> {
  // Both schema versions share a proto package; compile them into separate out dirs so
  // the binary can hold "the old plugin's view" and "the new engine's view" side by side.
  for v in ["v1", "v2"] {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join(v);
    std::fs::create_dir_all(&out)?;
    prost_build::Config::new()
      .out_dir(&out)
      .compile_protos(&[format!("proto/{v}/slice.proto")], &[format!("proto/{v}")])?;
  }
  Ok(())
}
