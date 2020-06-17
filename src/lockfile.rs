use std::cell::Cell;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use crate::errors::{PidgitError, Result};

#[derive(Debug)]
pub struct Lockfile {
  path:      PathBuf,
  lock_path: PathBuf,
  locked:    Cell<bool>,
}

// This is just a wrapper around an open file. Maybe later I want to improve
// this somewhat.
#[derive(Debug)]
pub struct FileLock<'l> {
  file:     File,
  lockfile: &'l Lockfile,
}

impl Lockfile {
  pub fn new(path: PathBuf) -> Self {
    let mut name = path.clone().into_os_string();
    name.push(".lock");

    Lockfile {
      path,
      lock_path: PathBuf::from(name),
      locked: Cell::new(false),
    }
  }

  pub fn lock(&self) -> Result<FileLock> {
    let file = OpenOptions::new()
      .write(true)
      .create_new(true)
      .open(&self.lock_path)
      .map_err(|e| PidgitError::Lock(self.lock_path.clone(), e))?;

    self.locked.set(true);

    Ok(FileLock {
      file,
      lockfile: &self,
    })
  }

  pub fn is_locked(&self) -> bool {
    self.locked.get()
  }

  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

impl<'l> FileLock<'l> {
  pub fn commit(mut self) -> Result<()> {
    use std::io::Write;
    // write this file out to its name, minus .lock, then drop ourselves
    self.file.flush()?;
    std::fs::rename(&self.lockfile.lock_path, &self.lockfile.path)?;
    self.lockfile.locked.set(false);
    Ok(())
  }

  pub fn rollback(self) -> Result<()> {
    std::fs::remove_file(&self.lockfile.lock_path)?;
    self.lockfile.locked.set(false);
    Ok(())
  }
}

impl<'l> std::io::Read for FileLock<'l> {
  fn read(
    &mut self,
    buf: &mut [u8],
  ) -> std::result::Result<usize, std::io::Error> {
    self.file.read(buf)
  }
}

impl<'l> std::io::Write for FileLock<'l> {
  fn write(&mut self, buf: &[u8]) -> std::result::Result<usize, std::io::Error> {
    self.file.write(buf)
  }

  fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
    self.file.flush()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_prelude::*;
  use std::ffi::OsString;
  use std::io::prelude::*;

  fn new_lockfile(dir: &TempDir, name: &str) -> Lockfile {
    let f = dir.child(name);
    Lockfile::new(f.path().to_path_buf())
  }

  #[test]
  fn create() {
    let d = tempdir();
    let lockfile = new_lockfile(&d, "index");
    assert_eq!(
      lockfile.path.file_name(),
      Some(OsString::from("index").as_os_str())
    );

    assert_eq!(
      lockfile.lock_path.file_name(),
      Some(OsString::from("index.lock").as_os_str()),
    );
  }

  #[test]
  fn lock() {
    let d = tempdir();
    let lockfile = new_lockfile(&d, "foo");
    let locked = lockfile.lock();

    assert!(locked.is_ok());

    // second should fail
    let relock = lockfile.lock();
    assert!(relock.is_err());
  }

  #[test]
  fn write_commit() {
    let d = tempdir();
    let lockfile = new_lockfile(&d, "index");
    let mut lock = lockfile.lock().unwrap();
    assert!(lockfile.is_locked());

    assert_eq!(lockfile.lock_path.is_file(), true, "locked a file");

    lock.write_all(b"hello\n").unwrap();
    lock.commit().expect("could not commit lockfile");

    assert!(!lockfile.is_locked());

    let mut s = String::new();
    File::open(&lockfile.path)
      .unwrap()
      .read_to_string(&mut s)
      .unwrap();

    assert_eq!(s, "hello\n", "content is correct in real file");
    assert_eq!(lockfile.lock_path.is_file(), false, "locked file is gone");
  }

  #[test]
  fn write_rollback() {
    let d = tempdir();
    let lockfile = new_lockfile(&d, "head");
    let mut lock = lockfile.lock().unwrap();
    assert!(lockfile.is_locked());

    lock.write_all(b"hello\n").unwrap();
    lock.commit().expect("could not commit lockfile");

    assert!(!lockfile.is_locked());

    let mut s = String::new();
    File::open(&lockfile.path)
      .unwrap()
      .read_to_string(&mut s)
      .unwrap();

    assert_eq!(s, "hello\n", "content is correct in real file");

    let mut lock = lockfile.lock().unwrap();
    lock.write_all(b"goodbye\n").unwrap();
    lock.rollback().expect("could not roll back lockfile");

    assert!(!lockfile.is_locked());

    let mut s = String::new();
    File::open(&lockfile.path)
      .unwrap()
      .read_to_string(&mut s)
      .unwrap();

    assert_eq!(s, "hello\n", "file content did not change");
  }
}
