//! Work the engine runs beside the frame stream (ADR 0062): a transport's exchange loop, a
//! verification run and a broker publication. Each must go on while the dispatcher keeps answering
//! frames, and the kernel starts no thread for it. It asks its embedding, through [`Background`],
//! which an embedding registers with [`super::Engine::register_background`]. An engine with none
//! does not declare the `background` capability, and refuses the three operations that need it by
//! name.
//!
//! [`Threads`] is the `std::thread` implementation every native embedding registers. It is the only
//! code in the kernel that starts a thread, and `tests/boundary.rs` holds the rest of the kernel
//! to that.

use std::fmt;
use std::thread;

/// What an embedding gives the kernel to run work beside the frame stream.
pub trait Background: Send + Sync {
  /// Start `task` and return at once. `name` says what the task is, for diagnostics: [`Threads`]
  /// names its thread after it.
  fn spawn(&self, name: &str, task: Box<dyn FnOnce() + Send + 'static>) -> Result<Task, SpawnError>;
}

/// A started task, which the kernel waits on once the work it belongs to has ended.
pub struct Task {
  join: Box<dyn FnOnce() -> Result<(), Panicked> + Send>,
}

impl Task {
  /// A task whose `join` waits as `join` does, and says whether the task panicked.
  pub fn new(join: impl FnOnce() -> Result<(), Panicked> + Send + 'static) -> Self {
    Task { join: Box::new(join) }
  }

  /// Wait for the task to end.
  pub fn join(self) -> Result<(), Panicked> {
    (self.join)()
  }
}

impl fmt::Debug for Task {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("Task")
  }
}

/// A task that ended by panicking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panicked;

/// Why a task could not be started: the embedding's own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnError(pub String);

impl fmt::Display for SpawnError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

impl std::error::Error for SpawnError {}

/// One named `std::thread` per task.
#[derive(Debug, Default, Clone, Copy)]
pub struct Threads;

impl Background for Threads {
  fn spawn(&self, name: &str, task: Box<dyn FnOnce() + Send + 'static>) -> Result<Task, SpawnError> {
    let handle = thread::Builder::new()
      .name(name.to_string())
      .spawn(task)
      .map_err(|err| SpawnError(format!("could not start a thread for {name}: {err}")))?;
    Ok(Task::new(move || handle.join().map_err(|_| Panicked)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;
  use std::sync::atomic::{AtomicBool, Ordering};

  #[test]
  fn threads_run_the_task_and_join_it() {
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let task = Threads
      .spawn("test task", Box::new(move || flag.store(true, Ordering::SeqCst)))
      .expect("a thread starts");
    assert_eq!(task.join(), Ok(()));
    assert!(ran.load(Ordering::SeqCst));
  }

  #[test]
  fn a_panicking_task_joins_as_panicked() {
    let task = Threads
      .spawn("panicking task", Box::new(|| panic!("on purpose")))
      .expect("a thread starts");
    assert_eq!(task.join(), Err(Panicked));
  }
}
