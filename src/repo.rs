mod grefs;
mod status;
pub use grefs::Grefs;
pub use status::{ChangeType, Status};

use flate2::{write::ZlibEncoder, Compression};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::OsString;
use std::fs::{DirBuilder, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use crate::index::Index;
use crate::object::{Blob, Commit, Object, Person, Tree};
use crate::prelude::*;

const GIT_DIR_NAME: &str = ".pidgit";
const HEAD: &str = "ref: refs/heads/main\n";
const CONFIG: &str = "\
[core]
\trepositoryformatversion = 0
\tfilemode = true
\tbare = false
\tlogallrefupdates = true
\tignorecase = true
\tprecomposeunicode = true
";

#[derive(Debug)]
pub struct Repository {
  workspace: Workspace,
  git_dir:   PathBuf,
  index:     RefCell<Index>,
  grefs:     RefCell<Grefs>,
}

#[derive(Debug)]
pub struct Workspace {
  path:     PathBuf,
  ignore:   HashSet<OsString>,
  ftignore: HashSet<OsString>,
}

// TODO for later
fn default_ignore() -> HashSet<OsString> {
  let mut ignore: HashSet<OsString> = HashSet::new();
  ignore.insert(".git".into());
  ignore.insert(".pidgit".into());
  ignore.insert("target".into());
  ignore.insert(".DS_Store".into());
  ignore
}

fn default_ftignore() -> HashSet<OsString> {
  let mut ftignore: HashSet<OsString> = HashSet::new();
  ftignore.insert("swp".into());
  ftignore.insert("swo".into());
  ftignore
}

impl Repository {
  // these paths must be canonicalized
  fn new(work_dir: &Path, git_dir: &Path) -> Result<Self> {
    let workspace = Workspace {
      path:     work_dir.to_path_buf(),
      ignore:   default_ignore(),
      ftignore: default_ftignore(),
    };

    let mut index = Index::new(git_dir.join("index"));
    index.load()?;

    Ok(Repository {
      workspace,
      git_dir: git_dir.to_path_buf(),
      index: RefCell::new(index),
      grefs: RefCell::new(Grefs::new(git_dir.to_path_buf())),
    })
  }

  pub fn from_work_tree(dir: &Path) -> Result<Self> {
    if !dir.is_dir() {
      return Err(PidgitError::Generic(format!(
        "cannot instantiate repo from working tree: {} is not a directory",
        dir.display()
      )));
    }

    Self::new(&dir.canonicalize()?, &dir.join(GIT_DIR_NAME))
  }

  pub fn from_git_dir(git_dir: &Path) -> Result<Self> {
    let path = git_dir.canonicalize()?;

    let parent = path.parent().ok_or_else(|| {
      PidgitError::Generic(format!(
        "cannot resolve git_dir: {} has no parent",
        path.display()
      ))
    })?;

    Self::new(parent, &path)
  }

  // We need to make, inside the current directory:
  // .pidgit/
  //    HEAD
  //    config
  //    index
  //    objects/
  //    refs/{heads,tags,remotes}/
  pub fn create_empty(root: &Path) -> Result<Self> {
    let git_dir = root.canonicalize()?.join(GIT_DIR_NAME);
    DirBuilder::new().create(&git_dir)?;

    let repo = Self::new(root, &git_dir)?;

    // HEAD
    let mut head = repo.create_file("HEAD")?;
    head.write_all(HEAD.as_bytes())?;

    // config
    let mut config = repo.create_file("config")?;
    config.write_all(CONFIG.as_bytes())?;

    // object dir
    repo.create_dir("objects")?;

    // refs
    repo.create_dir("refs/heads")?;
    repo.create_dir("refs/tags")?;
    repo.create_dir("refs/remotes")?;

    repo.index.borrow_mut().force_write()?;

    Ok(repo)
  }

  pub fn git_dir(&self) -> &PathBuf {
    &self.git_dir
  }

  pub fn workspace(&self) -> &Workspace {
    &self.workspace
  }

  pub fn grefs(&self) -> Ref<Grefs> {
    self.grefs.borrow()
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

  fn path_exists<P>(&self, path: P) -> bool
  where
    P: AsRef<Path> + std::fmt::Debug,
  {
    self.git_dir().join(path).is_file()
  }

  pub fn object_for_sha(&self, sha: &str) -> Result<Object> {
    Object::from_git_db(&self.path_for_sha(sha))
  }

  // NB returns an absolute path!
  pub fn path_for_sha(&self, sha: &str) -> PathBuf {
    let (first, rest) = sha.split_at(2);
    self.git_dir.join(format!("objects/{}/{}", first, rest))
  }

  pub fn write_object(&self, obj: &dyn GitObject) -> Result<()> {
    let path = self.path_for_sha(&obj.sha().hexdigest());

    // I am ignoring, here, the possibility that this path exists and might
    // somehow conflict??
    if path.is_file() {
      return Ok(());
    }

    // create parent dir!
    std::fs::create_dir_all(path.parent().unwrap())?;

    let file = File::create(&path)
      .expect(&format!("error creating path {}", path.display()));

    let mut e = ZlibEncoder::new(file, Compression::default());

    e.write_all(&obj.header())?;
    e.write_all(&obj.raw_content())?;
    e.finish()?;

    Ok(())
  }

  pub fn resolve_object(&self, name: &str) -> Result<Object> {
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

  fn resolve_ref(&self, refstr: &str) -> Result<Object> {
    let sha = self.grefs().resolve(refstr)?;
    self.resolve_sha(&sha)
  }

  fn resolve_sha(&self, sha: &str) -> Result<Object> {
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
          .to_str()
          .unwrap()
          .starts_with(&rest)
      })
      .collect::<Vec<_>>();

    match paths.len() {
      0 => not_found,
      1 => Object::from_git_db(&paths[0]),
      _ => Err(PidgitError::ObjectNotFound(format!("{} is ambiguous", sha))),
    }
  }

  pub fn index(&self) -> Ref<Index> {
    self.index.borrow()
  }

  pub fn index_mut(&self) -> RefMut<Index> {
    self.index.borrow_mut()
  }

  pub fn write_index(&self) -> Result<()> {
    self.index.borrow_mut().write()?;
    Ok(())
  }

  pub fn status(&self) -> Result<Status> {
    Status::generate(&self)
  }

  pub fn write_tree(&self, tree: &Tree) -> Result<()> {
    tree.traverse(&|t| self.write_object(t))
  }

  pub fn head(&self) -> Option<Commit> {
    self.resolve_ref("HEAD").and_then(|c| c.as_commit()).ok()
  }

  pub fn commit(
    &self,
    message: &str,
    author: Person,
    committer: Person,
  ) -> Result<Commit> {
    let head = self.resolve_object("HEAD").ok();

    let parents = if let Some(head) = head {
      vec![head.into_inner().sha().hexdigest()]
    } else {
      vec![]
    };

    let mut msg = message.to_string();

    if !msg.ends_with("\n") {
      msg.push_str("\n");
    }

    let tree = Tree::from(self.index());

    let commit = Commit {
      tree: tree.sha().hexdigest(),
      parent_shas: parents,
      author,
      committer,
      message: msg,
      content: None,
    };

    // we write the tree, then write the commit.
    self.write_tree(&tree)?;
    self.write_object(&commit)?;
    self.grefs().update_head(&commit.sha())?;

    Ok(commit)
  }
}

impl Workspace {
  pub fn root(&self) -> &PathBuf {
    &self.path
  }

  // give a path relative to the top of the work tree, canonicalize it.
  pub fn canonicalize<P>(&self, path: &P) -> PathBuf
  where
    P: AsRef<Path>,
  {
    self.path.join(path)
  }

  pub fn list_files(&self) -> Result<BTreeSet<OsString>> {
    self.list_files_from_base(&self.path)
  }

  pub fn list_files_from_base(
    &self,
    base: &PathBuf,
  ) -> Result<BTreeSet<OsString>> {
    let mut entries = BTreeSet::new();

    let list = self.list_dir(base)?;

    for (pathstr, stat) in list {
      if stat.is_dir() {
        entries.extend(self.list_files_from_base(&self.path.join(pathstr))?);
      } else {
        entries.insert(pathstr);
      }
    }

    Ok(entries)
  }

  pub fn list_dir(
    &self,
    raw_base: &PathBuf,
  ) -> Result<BTreeMap<OsString, std::fs::Metadata>> {
    let base = if raw_base.is_relative() {
      self.canonicalize(&raw_base)
    } else {
      raw_base.clone()
    };

    if !base.exists() {
      return Err(PidgitError::PathspecNotFound(
        base.as_os_str().to_os_string(),
      ));
    }

    let mut ret = BTreeMap::new();

    let relativize = |p: &PathBuf| {
      p.canonicalize()
        .expect("bad canonicalize")
        .strip_prefix(&self.path)
        .unwrap()
        .to_path_buf()
    };

    if base.is_file() {
      ret.insert(relativize(&base).into(), base.metadata()?);
      return Ok(ret);
    }

    for e in std::fs::read_dir(base)?.filter_map(std::result::Result::ok) {
      let path = e.path();

      if self.ignore.contains(path.file_name().unwrap()) {
        continue;
      }

      if let Some(ext) = path.extension() {
        if self.ftignore.contains(ext) {
          continue;
        }
      }

      ret.insert(relativize(&path).into(), path.metadata()?);
    }

    Ok(ret)
  }

  pub fn stat(&self, relpath: &PathBuf) -> Result<std::fs::Metadata> {
    Ok(self.canonicalize(relpath).metadata()?)
  }

  pub fn read_blob<P>(&self, relpath: &P) -> Result<Blob>
  where
    P: AsRef<Path>,
  {
    Blob::from_path(&self.canonicalize(relpath))
  }
}
