use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::File;
use std::fs::Metadata;
use std::io::{prelude::*, BufWriter, Cursor};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use bit_vec::BitVec;

use crate::prelude::*;
use crate::Lockfile;

const INDEX_VERSION: u32 = 2;
const MAX_PATH_SIZE: u16 = 0xfff;

pub struct Index {
  version:  u32,
  changed:  bool,
  entries:  BTreeMap<OsString, IndexEntry>,
  parents:  HashMap<OsString, HashSet<OsString>>,
  lockfile: Lockfile,
}

pub struct IndexEntry {
  meta:     EntryMeta,
  pub sha:  Sha,
  flags:    EntryFlags,
  pub name: OsString,
  changed:  bool,
}

#[derive(Debug)]
pub struct EntryFlags(BitVec);

#[derive(Eq, PartialEq)]
pub struct EntryMeta {
  ctime_sec:  u32,
  ctime_nano: u32,
  mtime_sec:  u32,
  mtime_nano: u32,
  dev:        u32,
  ino:        u32,
  mode:       u32,
  uid:        u32,
  gid:        u32,
  size:       u32,
}

impl fmt::Debug for Index {
  #[rustfmt::skip]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Index")
      .field("version", &self.version)
      .field("changed", &self.changed)
      .field("lockfile", &self.lockfile.path())
      .field("entries", &self.entries)
      .finish()
  }
}

impl fmt::Debug for IndexEntry {
  #[rustfmt::skip]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("IndexEntry")
      .field("name", &self.name)
      .field("sha", &self.sha)
      .field("flags", &self.flags)
      .field("meta",&self.meta)
      .finish()
  }
}

impl fmt::Debug for EntryMeta {
  #[rustfmt::skip]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("EntryMeta")
      .field("ctime",&format_args!("{}.{}", self.ctime_sec, self.ctime_nano))
      .field("mtime",&format_args!("{}.{}", self.mtime_sec, self.mtime_nano))
      .field("size", &self.size)
      .finish()
  }
}

fn index_error(s: &str) -> Result<()> {
  Err(PidgitError::Index(s.to_string()))
}

fn key_for_path(path: &PathBuf) -> OsString {
  path.into()
}

impl Index {
  pub fn new(path: PathBuf) -> Self {
    let lockfile = Lockfile::new(path);
    Self {
      version: INDEX_VERSION,
      changed: false,
      entries: BTreeMap::new(),
      parents: HashMap::new(),
      lockfile,
    }
  }

  pub fn reload(&mut self) -> Result<()> {
    self.entries = BTreeMap::new();
    self.parents = HashMap::new();
    self.load()
  }

  // parse this, based on
  // https://github.com/git/git/blob/master/Documentation/technical/index-format.txt
  // Here, I am ignoring extensions entirely!
  pub fn load(&mut self) -> Result<()> {
    if !self.lockfile.path().exists() {
      return Ok(()); // we're already an empty index!
    }

    if self.lockfile.is_locked() {
      return index_error("index file is locked; cannot read");
    }

    let mut raw = vec![];
    File::open(self.lockfile.path())?.read_to_end(&mut raw)?;

    let mut reader = Cursor::new(raw);

    let mut buf32 = [0u8; 4];

    // - A 12-byte header consisting of:

    // 4-byte signature: the signature is { 'D', 'I', 'R', 'C' }

    reader.read_exact(&mut buf32)?;

    if std::str::from_utf8(&buf32)? != "DIRC" {
      return index_error("malformed index header");
    }

    // 4-byte version number: The current supported versions are 2, 3 and 4.
    reader.read_exact(&mut buf32)?;
    let version = u32::from_be_bytes(buf32);

    if version != 2 && version != 3 {
      return index_error(&format!(
        "unsupported index version (want 2/3, have {})",
        version
      ));
    }

    self.version = version;

    // 32-bit number of index entries.
    reader.read_exact(&mut buf32)?;
    let num_entries = u32::from_be_bytes(buf32);

    // A number of sorted index entries
    // Index entry
    // Index entries are sorted in ascending order on the name field,
    // interpreted as a string of unsigned bytes (i.e. memcmp() order, no
    // localization, no special casing of directory separator '/'). Entries
    // with the same name are sorted by their stage field.
    for _ in 0..num_entries {
      let start_pos = reader.position();

      // 32-bit ctime seconds, the last time a file's metadata changed
      reader.read_exact(&mut buf32)?;
      let ctime_sec = u32::from_be_bytes(buf32);

      // 32-bit ctime nanosecond fractions
      reader.read_exact(&mut buf32)?;
      let ctime_nano = u32::from_be_bytes(buf32);

      // 32-bit mtime seconds, the last time a file's data changed
      reader.read_exact(&mut buf32)?;
      let mtime_sec = u32::from_be_bytes(buf32);

      // 32-bit mtime nanosecond fractions
      reader.read_exact(&mut buf32)?;
      let mtime_nano = u32::from_be_bytes(buf32);

      // 32-bit dev
      reader.read_exact(&mut buf32)?;
      let dev = u32::from_be_bytes(buf32);

      // 32-bit ino
      reader.read_exact(&mut buf32)?;
      let ino = u32::from_be_bytes(buf32);

      // 32-bit mode, split into (high to low bits)
      //   4-bit object type
      //     valid values in binary are 1000 (regular file), 1010 (symbolic link)
      //     and 1110 (gitlink)
      //   3-bit unused
      //   9-bit unix permission. Only 0755 and 0644 are valid for regular files.
      //   Symbolic links and gitlinks have value 0 in this field.
      reader.read_exact(&mut buf32)?;
      let mode = u32::from_be_bytes(buf32);

      // 32-bit uid
      reader.read_exact(&mut buf32)?;
      let uid = u32::from_be_bytes(buf32);

      // 32-bit gid
      reader.read_exact(&mut buf32)?;
      let gid = u32::from_be_bytes(buf32);

      // 32-bit file size (truncated to 32 bits)
      reader.read_exact(&mut buf32)?;
      let size = u32::from_be_bytes(buf32);

      // 160-bit SHA-1 for the represented object
      let mut sha = [0u8; sha1::DIGEST_LENGTH];
      reader.read_exact(&mut sha)?;

      // A 16-bit 'flags' field split into (high to low bits)
      //   1-bit assume-valid flag
      //   1-bit extended flag (must be zero in version 2)
      //   2-bit stage (during merge)
      //   12-bit name length if the length is less than 0xFFF; otherwise 0xFFF
      //   is stored in this field.
      let mut flagbuf = [0u8; 2];
      reader.read_exact(&mut flagbuf)?;
      let flags = EntryFlags::from(&flagbuf);

      // (Version 3 or later) A 16-bit field, only applicable if the
      // "extended flag" above is 1, split into (high to low bits).
      //   1-bit reserved for future
      //   1-bit skip-worktree flag (used by sparse checkout)
      //   1-bit intent-to-add flag (used by "git add -N")
      //   13-bit unused, must be zero
      if flags.is_extended() {
        // skip the extended flags, for now
        reader.seek(std::io::SeekFrom::Current(2))?;
      }

      // Entry path name (variable length) relative to top level directory
      let mut namebuf = vec![];
      reader.read_until(b'\0', &mut namebuf)?;
      namebuf.pop();
      let name = OsString::from_vec(namebuf);

      // 1-8 nul bytes as necessary to pad the entry to a multiple of eight bytes
      let len = reader.position() - start_pos;
      if len % 8 > 0 {
        let padding = 8 - len % 8;
        reader.seek(std::io::SeekFrom::Current(padding as i64))?;
      }

      let meta = EntryMeta {
        ctime_sec,
        ctime_nano,
        mtime_sec,
        mtime_nano,
        dev,
        ino,
        mode,
        uid,
        gid,
        size,
      };

      self.add(IndexEntry {
        meta,
        sha: sha.into(),
        flags,
        name,
        changed: false,
      });
    }

    // we haven't _actually_ changed
    self.changed = false;

    Ok(())
  }

  fn has_changed(&self) -> bool {
    self.changed || self.entries.values().any(|e| e.changed)
  }

  pub fn write(&self) -> Result<()> {
    if !self.has_changed() {
      // nothing to do!
      return Ok(());
    }

    self.force_write()
  }

  // write the index, without checking if it's changed first.
  pub fn force_write(&self) -> Result<()> {
    let lock = self.lockfile.lock()?;

    // TODO: flock this or something
    let mut writer = BufWriter::new(lock);
    let mut sha = sha1::Sha1::new();

    let mut header: Vec<u8> = Vec::with_capacity(12);
    header.extend("DIRC".as_bytes());
    header.extend(self.version.to_be_bytes().iter());
    header.extend(self.num_entries().to_be_bytes().iter());

    writer.write(&header)?;
    sha.update(&header);

    for entry in self.entries.values() {
      let bytes = entry.as_bytes();
      writer.write(&bytes)?;
      sha.update(&bytes);
    }

    // last 20 bytes is the sha of this content
    writer.write(&sha.digest().bytes())?;

    writer
      .into_inner()
      .expect("couldn't unwrap bufwriter")
      .commit()?;

    Ok(())
  }

  pub fn add(&mut self, entry: IndexEntry) {
    self.changed = true;
    self.remove_conflicts(&entry);

    for parent in entry.parents() {
      let k = key_for_path(&parent);
      let mut set = self.parents.get_mut(&k);

      if set.is_none() {
        self.parents.insert(k.clone(), HashSet::new());
        set = self.parents.get_mut(&k);
      }

      set.unwrap().insert(entry.name.clone());
    }

    self.entries.insert(entry.name.clone(), entry);
  }

  fn remove_conflicts(&mut self, entry: &IndexEntry) {
    for parent in entry.parents() {
      self.remove_entry(&key_for_path(&parent));
    }

    self.remove_children(entry);
  }

  fn remove_entry(&mut self, key: &OsStr) {
    let entry = self.entries.remove(key);

    if let Some(entry) = entry {
      for parent in entry.parents() {
        let k = key_for_path(&parent);
        let children = self.parents.get_mut(&k).unwrap();
        children.remove(&entry.name);

        if children.is_empty() {
          self.parents.remove(&k);
        }
      }
    }
  }

  fn remove_children(&mut self, entry: &IndexEntry) {
    if !self.parents.contains_key(&entry.name) {
      return;
    }

    let children = self.parents.get(&entry.name).unwrap().clone();

    for child in children.iter() {
      self.remove_entry(child);
    }
  }

  pub fn entries(&self) -> impl Iterator<Item = &IndexEntry> {
    self.entries.values()
  }

  pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut IndexEntry> {
    self.entries.values_mut()
  }

  pub fn keys(&self) -> impl Iterator<Item = &OsString> {
    self.entries.keys()
  }

  pub fn num_entries(&self) -> u32 {
    self.entries.len() as u32
  }

  pub fn is_tracked(&self, key: &OsStr) -> bool {
    self.is_tracked_file(key) || self.parents.contains_key(key)
  }

  pub fn is_tracked_file(&self, key: &OsStr) -> bool {
    self.entries.contains_key(key)
  }

  pub fn entry_for(&self, key: &OsStr) -> Option<&IndexEntry> {
    self.entries.get(key)
  }
}

impl IndexEntry {
  pub fn new(key: OsString, path: &PathBuf) -> Result<Self> {
    let meta = path.metadata()?;
    let sha = util::compute_sha_for_path(path, Some(&meta))?;

    Ok(Self::new_from_data(key, sha, meta))
  }

  pub fn new_from_data(name: OsString, sha: Sha, stat: Metadata) -> Self {
    let flags = EntryFlags::from_path(&name);
    let meta = EntryMeta::from(&stat);

    IndexEntry {
      meta,
      sha,
      flags,
      name,
      changed: false,
    }
  }

  pub fn as_bytes(&self) -> Vec<u8> {
    // 64 bytes is constant, plus a filename, so allow some room for that
    let mut ret = Vec::with_capacity(100);

    // I think this is probably not very efficient.
    ret.extend(self.meta.as_bytes());
    ret.extend(self.sha.bytes());
    ret.extend(self.flags.as_bytes().iter());
    ret.extend(self.name.as_os_str().as_bytes());
    ret.push(0);

    // 1-8 nul bytes as necessary to pad the entry to a multiple of eight bytes
    while ret.len() % 8 > 0 {
      ret.push(0);
    }

    ret
  }

  pub fn parents(&self) -> Vec<PathBuf> {
    let path = PathBuf::from(&self.name);
    let mut parents = path
      .ancestors()
      .skip(1)
      .map(|p| p.to_path_buf())
      .collect::<Vec<_>>();

    parents.pop(); // remove empty path
    parents.reverse();

    parents
  }

  pub fn mode(&self) -> u32 {
    self.meta.mode
  }

  // not perfect, obviously
  pub fn matches_stat(&self, stat: &Metadata) -> bool {
    let other = EntryMeta::from(stat);
    self.meta.size == other.size && self.meta.mode == other.mode
  }

  // not perfect, obviously
  pub fn matches_time(&self, stat: &Metadata) -> bool {
    let other = EntryMeta::from(stat);
    let me = &self.meta;
    me.ctime_sec == other.ctime_sec
      && me.ctime_nano == other.ctime_nano
      && me.mtime_sec == other.mtime_sec
      && me.mtime_nano == other.mtime_nano
  }

  pub fn update_meta(&mut self, stat: &Metadata) {
    let other = EntryMeta::from(stat);
    self.meta = other;
    self.changed = true;
  }
}

impl EntryMeta {
  pub fn as_bytes(&self) -> Vec<u8> {
    let mut ret = vec![];
    ret.extend(self.ctime_sec.to_be_bytes().iter());
    ret.extend(self.ctime_nano.to_be_bytes().iter());
    ret.extend(self.mtime_sec.to_be_bytes().iter());
    ret.extend(self.mtime_nano.to_be_bytes().iter());
    ret.extend(self.dev.to_be_bytes().iter());
    ret.extend(self.ino.to_be_bytes().iter());
    ret.extend(self.mode.to_be_bytes().iter());
    ret.extend(self.uid.to_be_bytes().iter());
    ret.extend(self.gid.to_be_bytes().iter());
    ret.extend(self.size.to_be_bytes().iter());
    ret
  }
}

impl From<&Metadata> for EntryMeta {
  fn from(meta: &Metadata) -> Self {
    Self {
      ctime_sec:  meta.ctime() as u32,
      ctime_nano: meta.ctime_nsec() as u32,
      mtime_sec:  meta.mtime() as u32,
      mtime_nano: meta.mtime_nsec() as u32,
      dev:        meta.dev() as u32,
      ino:        meta.ino() as u32,
      mode:       meta.mode(), // XXX is this right?
      uid:        meta.uid(),
      gid:        meta.gid(),
      size:       meta.size() as u32,
    }
  }
}

// A 16-bit 'flags' field split into (high to low bits)
//   1-bit assume-valid flag
//   1-bit extended flag (must be zero in version 2)
//   2-bit stage (during merge)
//   12-bit name length if the length is less than 0xFFF; otherwise 0xFFF
//   is stored in this field.
impl EntryFlags {
  fn storage(&self) -> &BitVec {
    &self.0
  }

  pub fn as_bytes(&self) -> Vec<u8> {
    self.0.to_bytes()
  }

  pub fn from_path(path: &OsString) -> Self {
    let mut pathlen = path.len() as u16;

    if pathlen > MAX_PATH_SIZE {
      pathlen = MAX_PATH_SIZE;
    }

    // TODO: For now, I think storing just the name length is sufficient.
    let mut flags = BitVec::from_elem(16, false);
    let pathlen = BitVec::from_bytes(&pathlen.to_be_bytes());
    flags.or(&pathlen);
    Self(flags)
  }

  pub fn is_extended(&self) -> bool {
    self.storage().get(1).unwrap()
  }
}

impl From<&[u8; 2]> for EntryFlags {
  fn from(bytes: &[u8; 2]) -> Self {
    Self(BitVec::from_bytes(bytes))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_prelude::*;

  const EMPTY_SHA: &str = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";

  fn random_stat() -> std::fs::Metadata {
    let d = tempdir();
    let f = d.child("dummy");
    f.touch().unwrap();
    f.path().metadata().unwrap()
  }

  fn new_empty_index() -> Index {
    let f = tempdir().child("dummy").path().to_path_buf();
    Index::new(f)
  }

  fn index_with_entries(names: &[&str]) -> Index {
    let mut idx = new_empty_index();
    for name in names {
      idx.add(new_empty_entry(name));
    }

    idx
  }

  fn new_empty_entry(basename: &str) -> IndexEntry {
    IndexEntry::new_from_data(basename.into(), EMPTY_SHA.into(), random_stat())
  }

  #[test]
  fn entry_from_path() {
    let dir = tempdir();
    let f = dir.child("foo.txt");
    f.write_str("").unwrap();
    let entry = IndexEntry::new("foo.txt".into(), &f.path().to_path_buf())
      .expect("couldn't create entry");

    assert_eq!(entry.mode(), 0o100644);
    assert_eq!(entry.sha, EMPTY_SHA.into());
    assert!(entry.name.to_str().unwrap().ends_with("foo.txt"));
  }

  #[test]
  fn add_file() {
    let mut idx = new_empty_index();
    let entry = new_empty_entry("alice.txt");
    idx.add(entry);

    assert_eq!(idx.num_entries(), 1);
    assert_eq!(idx.changed, true);
  }

  #[test]
  fn replace_file_with_dir() {
    let mut idx = index_with_entries(&["alice.txt", "bob.txt"]);

    assert_eq!(idx.num_entries(), 2);
    assert_eq!(
      vec!["alice.txt", "bob.txt"],
      idx.entries.keys().collect::<Vec<_>>()
    );

    idx.add(new_empty_entry("alice.txt/nested.txt"));

    assert_eq!(idx.num_entries(), 2);
    assert_eq!(
      vec!["alice.txt/nested.txt", "bob.txt"],
      idx.entries.keys().collect::<Vec<_>>()
    );
  }

  #[test]
  fn replace_dir_with_file() {
    let mut idx = index_with_entries(&["alice.txt", "nested/bob.txt"]);

    idx.add(new_empty_entry("nested"));

    assert_eq!(idx.num_entries(), 2);
    assert_eq!(
      vec!["alice.txt", "nested"],
      idx.entries.keys().collect::<Vec<_>>()
    );
  }

  #[test]
  fn replace_dir_with_file_recursive() {
    let mut idx = index_with_entries(&[
      "alice.txt",
      "nested/bob.txt",
      "nested/inner/claire.txt",
    ]);

    let mut parent_keys = idx.parents.keys().collect::<Vec<_>>();
    parent_keys.sort();

    assert_eq!(vec!["nested", "nested/inner"], parent_keys);

    idx.add(new_empty_entry("nested"));

    assert_eq!(idx.num_entries(), 2);
    assert_eq!(
      vec!["alice.txt", "nested"],
      idx.entries.keys().collect::<Vec<_>>()
    );
    assert!(idx.parents.is_empty());
  }
}
