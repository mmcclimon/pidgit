use std::cell::RefMut;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::Metadata;
use std::path::PathBuf;

use crate::index::Index;
use crate::object::{PathEntry, TreeItem};
use crate::prelude::*;

// lifetime is bound to the repository
#[derive(Debug)]
struct InnerStatus<'r> {
  repo:           &'r Repository,
  index:          RefMut<'r, Index>,
  stats:          BTreeMap<OsString, Metadata>,
  untracked:      BTreeMap<OsString, ChangeType>,
  index_diff:     BTreeMap<OsString, ChangeType>,
  workspace_diff: BTreeMap<OsString, ChangeType>,
  head_diff:      BTreeMap<OsString, PathEntry>,
}

#[derive(Debug)]
pub struct Status {
  stats:          BTreeMap<OsString, Metadata>,
  untracked:      BTreeMap<OsString, ChangeType>,
  index_diff:     BTreeMap<OsString, ChangeType>,
  workspace_diff: BTreeMap<OsString, ChangeType>,
  head_diff:      BTreeMap<OsString, PathEntry>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
#[allow(unused)]
pub enum ChangeType {
  Modified,
  Deleted,
  Added,
  Untracked,
}

impl Status {
  pub fn generate(repo: &Repository) -> Result<Self> {
    let mut helper = InnerStatus::new(repo);
    helper.check()?;
    Ok(Self {
      untracked:      helper.untracked,
      index_diff:     helper.index_diff,
      workspace_diff: helper.workspace_diff,
      head_diff:      helper.head_diff,
      stats:          helper.stats,
    })
  }

  // accessors...it would be nice to provide something better than this.

  pub fn untracked(&self) -> &BTreeMap<OsString, ChangeType> {
    &self.untracked
  }

  pub fn has_untracked_changes(&self) -> bool {
    self.untracked.len() > 0
  }

  pub fn index_diff(&self) -> &BTreeMap<OsString, ChangeType> {
    &self.index_diff
  }

  pub fn has_index_changes(&self) -> bool {
    self.index_diff.len() > 0
  }

  pub fn workspace_diff(&self) -> &BTreeMap<OsString, ChangeType> {
    &self.workspace_diff
  }

  pub fn has_workspace_changes(&self) -> bool {
    self.workspace_diff.len() > 0
  }

  pub fn head_diff(&self) -> &BTreeMap<OsString, PathEntry> {
    &self.head_diff
  }

  pub fn stat_for(&self, key: &OsString) -> Option<&Metadata> {
    self.stats.get(key)
  }
}

impl<'r> InnerStatus<'r> {
  pub fn new(repo: &'r Repository) -> Self {
    Self {
      repo,
      index: repo.index.borrow_mut(),
      untracked: BTreeMap::new(),
      stats: BTreeMap::new(),
      head_diff: BTreeMap::new(),
      index_diff: BTreeMap::new(),
      workspace_diff: BTreeMap::new(),
    }
  }

  pub fn check(&mut self) -> Result<()> {
    let workspace = self.repo.workspace();

    self.scan_workspace(workspace.root())?;
    self.load_head()?;
    self.detect_changes();

    // update the index, in case any of the stats have changed
    self.index.write()?;

    Ok(())
  }

  fn scan_workspace(&mut self, base: &PathBuf) -> Result<()> {
    let ws = self.repo.workspace();
    for (path_str, stat) in ws.list_dir(base)? {
      let is_dir = stat.is_dir();

      if self.index.is_tracked(&path_str) {
        if is_dir {
          self.scan_workspace(&ws.canonicalize(&path_str))?;
        } else if stat.is_file() {
          self.stats.insert(path_str.clone(), stat.clone());
        }
      } else if self.is_trackable(&path_str, &stat) {
        let suffix = if is_dir {
          std::path::MAIN_SEPARATOR.to_string()
        } else {
          "".to_string()
        };

        let mut name = path_str.clone();
        name.push(suffix);

        self.untracked.insert(name, ChangeType::Untracked);
      }
    }

    Ok(())
  }

  // a path is trackable iff it contains a file somewhere inside it.
  fn is_trackable(&self, path: &OsString, stat: &Metadata) -> bool {
    if stat.is_file() {
      return !self.index.is_tracked(&path);
    }

    if !stat.is_dir() {
      return false;
    }

    let ws = self.repo.workspace();

    ws.list_dir(&path.into())
      .expect(&format!("could not list dir {:?}", path))
      .iter()
      .any(|(path, stat)| self.is_trackable(path, stat))
  }

  fn detect_changes(&mut self) {
    self.check_index();
    self.check_head();
  }

  fn check_index(&mut self) {
    // Check the working tree: for every file in the index, if our stat is
    // different than it, it's changed.
    for entry in self.index.entries_mut() {
      let path = &entry.name;
      let stat = self.stats.get(path);

      if stat.is_none() {
        self
          .workspace_diff
          .insert(path.clone(), ChangeType::Deleted);
        continue;
      }

      let stat = stat.unwrap();

      if !entry.matches_stat(stat) {
        self
          .workspace_diff
          .insert(path.clone(), ChangeType::Modified);
        continue;
      }

      if entry.matches_time(stat) {
        continue;
      }

      // Check the content
      let sha = util::compute_sha_for_path(
        &self.repo.workspace().canonicalize(path),
        Some(stat),
      )
      .expect("could not calculate sha")
      .hexdigest();

      if sha == entry.sha {
        // if we've gotten here, we know the index stat time is stale
        entry.update_meta(stat);
        continue;
      } else {
        self
          .workspace_diff
          .insert(path.clone(), ChangeType::Modified);
      }
    }
  }

  fn check_head(&mut self) {
    // now, check against the head
    for entry in self.index.entries() {
      let path = &entry.name;

      if !self.head_diff.contains_key(path) {
        self.index_diff.insert(path.clone(), ChangeType::Added);
        continue;
      }

      let have = self.head_diff.get(path).unwrap();

      if have.mode() != entry.mode() || have.sha() != entry.sha {
        self.index_diff.insert(path.clone(), ChangeType::Modified);
        continue;
      }
    }

    // check for files that exist in HEAD but aren't in the index
    for key in self.head_diff.keys() {
      if !self.index.is_tracked_file(key) {
        self.index_diff.insert(key.clone(), ChangeType::Deleted);
      }
    }
  }

  fn load_head(&mut self) -> Result<()> {
    let head = self.repo.head();
    if head.is_none() {
      return Ok(());
    }

    let tree = head.unwrap().tree().to_string();
    self.read_tree(&tree, &PathBuf::from(""))?;

    Ok(())
  }

  fn read_tree(&mut self, sha: &str, prefix: &PathBuf) -> Result<()> {
    let tree = self.repo.resolve_object(sha)?.as_tree()?;

    for (path, entry) in tree.entries() {
      if let TreeItem::Entry(e) = entry {
        let fullpath = prefix.join(path);
        if e.is_tree() {
          self.read_tree(e.sha(), &fullpath)?;
        } else {
          self.head_diff.insert(fullpath.into(), e.clone());
        }
      }
    }

    Ok(())
  }
}

impl ChangeType {
  pub fn display(&self) -> &'static str {
    match self {
      Self::Modified => "M",
      Self::Deleted => "D",
      Self::Added => "A",
      Self::Untracked => "?",
    }
  }

  pub fn long_display(&self) -> &'static str {
    match self {
      Self::Modified => "modified",
      Self::Deleted => "deleted",
      Self::Added => "new file",
      Self::Untracked => "untracked file",
    }
  }
}
