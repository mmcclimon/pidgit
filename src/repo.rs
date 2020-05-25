use flate2::{write::ZlibEncoder, Compression};
use std::fs::{DirBuilder, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use crate::index::Index;
use crate::object::RawObject;
use crate::{PidgitError, Result};

const GIT_DIR_NAME: &'static str = ".pidgit";

#[derive(Debug)]
pub struct Repository {
  work_tree: PathBuf,
  git_dir:   PathBuf,
}

impl Repository {
  pub fn from_work_tree(dir: &Path) -> Result<Self> {
    if !dir.is_dir() {
      return Err(PidgitError::Generic(format!(
        "cannot instantiate repo from working tree: {} is not a directory",
        dir.display()
      )));
    }

    Ok(Repository {
      work_tree: dir.to_path_buf(),
      git_dir:   dir.join(GIT_DIR_NAME),
    })
  }

  pub fn from_git_dir(git_dir: &Path) -> Result<Self> {
    let path = git_dir.canonicalize()?;

    let parent = path.parent().ok_or_else(|| {
      PidgitError::Generic(format!(
        "cannot resolve git_dir: {} has no parent",
        path.display()
      ))
    })?;

    Ok(Repository {
      work_tree: parent.to_path_buf(),
      git_dir:   git_dir.to_path_buf(),
    })
  }

  pub fn create_empty(root: &Path) -> Result<Self> {
    let git_dir = root.join(GIT_DIR_NAME);
    DirBuilder::new().create(&git_dir)?;

    Ok(Repository {
      work_tree: root.to_path_buf(),
      git_dir,
    })
  }

  pub fn work_tree(&self) -> &PathBuf {
    &self.work_tree
  }

  pub fn git_dir(&self) -> &PathBuf {
    &self.git_dir
  }

  // given a path relative to git_dir, create that file
  pub fn create_file<P>(&self, path: P) -> Result<File>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    File::create(self.git_dir.join(path)).map_err(|e| e.into())
  }

  // given a path relative to git_dir, create that file
  pub fn create_dir<P>(&self, path: P) -> Result<()>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    DirBuilder::new()
      .recursive(true)
      .create(self.git_dir.join(path))
      .map_err(|e| e.into())
  }

  // give it a path relative to .git_dir, read into a string
  pub fn read_file<P>(&self, path: P) -> Result<String>
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    let mut s = String::new();
    File::open(self.git_dir().join(path))?.read_to_string(&mut s)?;
    Ok(s.trim().to_string())
  }

  fn path_exists<P>(&self, path: P) -> bool
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    self.git_dir().join(path).is_file()
  }

  pub fn object_for_sha(&self, sha: &str) -> Result<RawObject> {
    RawObject::from_path(&self.path_for_sha(sha))
  }

  // NB returns an absolute path!
  pub fn path_for_sha(&self, sha: &str) -> PathBuf {
    let (first, rest) = sha.split_at(2);
    self.git_dir.join(format!("objects/{}/{}", first, rest))
  }

  pub fn write_object(&self, obj: &RawObject) -> Result<()> {
    let path = self.path_for_sha(&obj.sha().hexdigest());

    // create parent dir!
    std::fs::create_dir_all(path.parent().unwrap())?;

    let file = File::create(path)?;

    let mut e = ZlibEncoder::new(file, Compression::default());

    e.write_all(&obj.header())?;
    e.write_all(&obj.content())?;
    e.finish()?;

    Ok(())
  }

  pub fn resolve_object(&self, name: &str) -> Result<RawObject> {
    // this may get more smarts later
    let to_match = match name {
      "head" | "@" => "HEAD",
      _ => name,
    };

    // this algorithm directly from git rev-parse docs
    for prefix in &[".", "refs", "refs/tags", "refs/heads", "refs/remotes"] {
      let joined = format!("{}/{}", prefix, to_match);

      if self.path_exists(&joined) {
        return self.resolve_ref(&joined);
      }
    }

    // also check head of remotes
    let remote_head = format!("refs/remotes/{}/HEAD", to_match);
    if self.path_exists(&remote_head) {
      return self.resolve_ref(&remote_head);
    }

    // not found yet, assume a sha
    self.resolve_sha(to_match)
  }

  fn resolve_ref(&self, refstr: &str) -> Result<RawObject> {
    let raw = self.read_file(refstr)?;

    if raw.starts_with("ref: ") {
      let symref = raw.trim_start_matches("ref: ");
      self.resolve_ref(symref)
    } else {
      self.object_for_sha(&raw)
    }
  }

  fn resolve_sha(&self, sha: &str) -> Result<RawObject> {
    if sha.len() < 4 {
      return Err(PidgitError::ObjectNotFound(format!(
        "{} is too short to be a sha",
        sha
      )));
    }

    if sha.len() == 40 {
      return self.object_for_sha(sha);
    }

    let not_found = Err(PidgitError::ObjectNotFound(sha.to_string()));

    // we need to walk the objects dir
    let (prefix, rest) = sha.split_at(2);
    let base = self.git_dir.join(format!("objects/{}", &prefix));

    if !base.is_dir() {
      return not_found;
    }

    let paths = std::fs::read_dir(base)?
      .filter(|e| e.is_ok())
      .map(|e| e.unwrap().path())
      .filter(|path| {
        path
          .file_name()
          .unwrap()
          .to_string_lossy()
          .starts_with(&rest)
      })
      .collect::<Vec<_>>();

    match paths.len() {
      0 => not_found,
      1 => RawObject::from_path(&paths[0]),
      _ => Err(PidgitError::ObjectNotFound(format!("{} is ambiguous", sha))),
    }
  }

  pub fn index(&self) -> Result<Index> {
    Index::from_path(self.git_dir().join("index"))
  }
}
