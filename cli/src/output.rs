//! Writing to stdout and stderr without panicking when the reader has gone.
//!
//! `println!` panics when its write fails, and a closed pipe is the ordinary way a write fails:
//! `janus verify ... | head` closes stdout after ten lines. Rust ignores `SIGPIPE`, so the write
//! returns an error instead of killing the process, and `println!` turns that error into a panic
//! trace and an exit code nobody chose.
//!
//! Everything the CLI prints goes through [`outln!`] and [`errln!`] instead. When a stream has gone,
//! later writes to it are dropped and the command carries on: a verification still runs to its end,
//! and its exit code still says whether the provider passed, whoever stopped reading.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

static STDOUT_GONE: AtomicBool = AtomicBool::new(false);
static STDERR_GONE: AtomicBool = AtomicBool::new(false);

/// `println!`, minus the panic.
macro_rules! outln {
  () => {
    $crate::output::out(format_args!(""))
  };
  ($($arg:tt)*) => {
    $crate::output::out(format_args!($($arg)*))
  };
}

/// `eprintln!`, minus the panic.
macro_rules! errln {
  () => {
    $crate::output::err(format_args!(""))
  };
  ($($arg:tt)*) => {
    $crate::output::err(format_args!($($arg)*))
  };
}

pub fn out(line: std::fmt::Arguments) {
  write_line(&STDOUT_GONE, &mut std::io::stdout().lock(), line);
}

pub fn err(line: std::fmt::Arguments) {
  write_line(&STDERR_GONE, &mut std::io::stderr().lock(), line);
}

/// Bytes exactly as given, with no newline added: a canonical contract, a completion script.
pub fn out_bytes(bytes: &[u8]) {
  let mut stdout = std::io::stdout().lock();
  write_with(&STDOUT_GONE, || {
    stdout.write_all(bytes).and_then(|()| stdout.flush())
  });
}

/// A JSON document as a script reads it and a person can still read it.
pub fn pretty(document: &serde_json::Value) -> String {
  serde_json::to_string_pretty(document).unwrap_or_else(|_| document.to_string())
}

fn write_line(gone: &AtomicBool, stream: &mut impl Write, line: std::fmt::Arguments) {
  write_with(gone, || {
    stream.write_fmt(line)?;
    stream.write_all(b"\n")?;
    stream.flush()
  });
}

fn write_with(gone: &AtomicBool, write: impl FnOnce() -> std::io::Result<()>) {
  if gone.load(Ordering::Relaxed) {
    return;
  }
  if let Err(err) = write() {
    gone.store(true, Ordering::Relaxed);
    if err.kind() != std::io::ErrorKind::BrokenPipe {
      tracing::debug!(%err, "an output stream failed; nothing more will be written to it");
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A writer whose reader has gone, counting what it was asked to write.
  struct Closed(usize);

  impl Write for Closed {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
      self.0 += 1;
      Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> std::io::Result<()> {
      Ok(())
    }
  }

  #[test]
  fn a_closed_stream_is_written_to_once_and_then_left_alone() {
    let gone = AtomicBool::new(false);
    let mut closed = Closed(0);
    write_line(&gone, &mut closed, format_args!("first"));
    write_line(&gone, &mut closed, format_args!("second"));
    assert!(gone.load(Ordering::Relaxed));
    assert_eq!(closed.0, 1);
  }
}
