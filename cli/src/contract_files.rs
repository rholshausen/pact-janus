//! The native contract file writer (engine-protocol spec §8.2, ADR 0065): what `finalise`'s `write`
//! goes through. It opens the file, holds an exclusive lock on it from reading to writing, and
//! rewrites it in place; what it writes is the kernel's to decide, from what it read
//! ([`files::next_bytes`]). The lock is the operating system's, so it holds across processes: Jest's
//! and Vitest's workers and Gradle's forks each have an engine of their own, all writing one file.

use pact_janus_kernel::contract::files::{self, ContractWriter, Next};
use pact_janus_kernel::protocol::EngineError;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Writes contract files on this machine's filesystem.
pub struct FileContractWriter;

/// What went wrong under the lock: the file, or the kernel's answer to what it held.
enum Failure {
  Io(std::io::Error),
  Refused(EngineError),
}

impl From<std::io::Error> for Failure {
  fn from(err: std::io::Error) -> Self {
    Failure::Io(err)
  }
}

impl ContractWriter for FileContractWriter {
  fn update(&self, path: &str, next: &mut Next<'_>) -> Result<(), EngineError> {
    let file = Path::new(path);
    if !file.is_absolute() {
      return Err(EngineError::malformed_frame(format!(
        "write.path: '{path}' is not absolute; the engine resolves no path against a directory of its own"
      )));
    }
    if let Some(dir) = file.parent() {
      fs::create_dir_all(dir).map_err(|err| files::failed(path, &err))?;
    }
    // Not truncated on open: a writer waiting for the lock would read an emptied file.
    let mut handle = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .truncate(false)
      .open(file)
      .map_err(|err| files::failed(path, &err))?;
    handle.lock().map_err(|err| files::failed(path, &err))?;
    let written = rewrite(&mut handle, next);
    // Closing the file releases the lock too; unlocking first says when.
    let _ = handle.unlock();
    match written {
      Ok(()) => Ok(()),
      Err(Failure::Io(err)) => Err(files::failed(path, &err)),
      Err(Failure::Refused(err)) => Err(err),
    }
  }
}

/// Reads the locked file, and replaces its contents with what `next` makes of them.
fn rewrite(file: &mut File, next: &mut Next<'_>) -> Result<(), Failure> {
  let mut existing = Vec::new();
  file.read_to_end(&mut existing)?;
  let bytes = next(&existing).map_err(Failure::Refused)?;
  file.seek(SeekFrom::Start(0))?;
  file.write_all(&bytes)?;
  file.set_len(bytes.len() as u64)?;
  file.sync_all()?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;
  use std::sync::{Arc, Barrier};

  /// A directory of this test's own, emptied.
  fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("janus-contract-files-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  fn append(line: &'static str) -> impl FnMut(&[u8]) -> Result<Vec<u8>, EngineError> {
    move |existing: &[u8]| {
      let mut bytes = existing.to_vec();
      bytes.extend_from_slice(line.as_bytes());
      Ok(bytes)
    }
  }

  #[test]
  fn writers_racing_on_one_file_each_see_what_the_last_wrote() {
    let path = scratch("racing")
      .join("nested")
      .join("c.janus.json")
      .to_string_lossy()
      .into_owned();
    let writers = 16;
    let start = Arc::new(Barrier::new(writers));
    let handles: Vec<_> = (0..writers)
      .map(|_| {
        let (path, start) = (path.clone(), Arc::clone(&start));
        std::thread::spawn(move || {
          start.wait();
          FileContractWriter.update(&path, &mut append("x\n")).unwrap();
        })
      })
      .collect();
    for handle in handles {
      handle.join().unwrap();
    }
    assert_eq!(fs::read_to_string(&path).unwrap(), "x\n".repeat(writers));
  }

  #[test]
  fn a_refusal_leaves_the_file_as_it_was() {
    let path = scratch("refusal").join("c.janus.json");
    fs::write(&path, "kept").unwrap();
    let err = FileContractWriter
      .update(&path.to_string_lossy(), &mut |_| {
        Err(EngineError::malformed_frame("no"))
      })
      .unwrap_err();
    assert_eq!(err.code, "malformed-frame");
    assert_eq!(fs::read_to_string(&path).unwrap(), "kept");
  }

  #[test]
  fn shorter_bytes_leave_nothing_of_the_longer_behind() {
    let path = scratch("shorter").join("c.janus.json");
    fs::write(&path, "a much longer document").unwrap();
    FileContractWriter
      .update(&path.to_string_lossy(), &mut |_| Ok(b"short".to_vec()))
      .unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "short");
  }

  #[test]
  fn a_relative_path_is_refused() {
    let err = FileContractWriter
      .update("contracts/c.janus.json", &mut |_| Ok(vec![]))
      .unwrap_err();
    assert_eq!(err.code, "malformed-frame");
  }
}
