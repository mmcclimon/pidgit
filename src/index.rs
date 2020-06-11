use std::collections::BTreeMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, BufWriter, Cursor};
use std::path::{Path, PathBuf};

use crate::prelude::*;

const MAX_PATH_SIZE: usize = 0xfff;

#[derive(Debug)]
pub struct Index {
  version: u32,
  entries: BTreeMap<String, IndexEntry>,
  changed: bool,
}

pub struct IndexEntry {
  ctime_sec:  u32,
  ctime_nano: u32,
  mtime_sec:  u32,
  mtime_nano: u32,
  dev:        u32,
  ino:        u32,
  pub mode:   u32,
  uid:        u32,
  gid:        u32,
  size:       u32,
  pub sha:    String,
  flags:      u16,
  pub name:   String,
}

impl fmt::Debug for IndexEntry {
  #[rustfmt::skip]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("IndexEntry")
      .field("name", &self.name)
      .field("sha", &self.sha)
      .field("ctime",&format_args!("{}.{}", self.ctime_sec, self.ctime_nano))
      .field("mtime",&format_args!("{}.{}", self.mtime_sec, self.mtime_nano))
      .field("size", &self.size)
      .field("flags", &format_args!("{:018b}", &self.flags))
      .finish()
  }
}

fn index_error(s: &str) -> Result<Index> {
  Err(PidgitError::Index(s.to_string()))
}

impl Index {
  // parse this, based on
  // https://github.com/git/git/blob/master/Documentation/technical/index-format.txt
  // Here, I am ignoring extensions entirely!
  pub fn from_path<P>(path: P) -> Result<Self>
  where
    P: AsRef<Path> + fmt::Debug,
  {
    let mut raw = vec![];
    File::open(&path)?.read_to_end(&mut raw)?;

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

    if version != 2 {
      return index_error("unsupported index version");
    }

    // 32-bit number of index entries.
    reader.read_exact(&mut buf32)?;
    let num_entries = u32::from_be_bytes(buf32);

    // A number of sorted index entries
    let mut entries = BTreeMap::new();

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
      let flags = u16::from_be_bytes(flagbuf);

      // Entry path name (variable length) relative to top level directory
      let mut namebuf = vec![];
      reader.read_until(b'\0', &mut namebuf)?;
      namebuf.pop();
      let name = String::from_utf8(namebuf)?;

      // 1-8 nul bytes as necessary to pad the entry to a multiple of eight bytes
      let len = reader.position() - start_pos;
      if len % 8 > 0 {
        let padding = 8 - len % 8;
        reader.seek(std::io::SeekFrom::Current(padding as i64))?;
      }

      entries.insert(
        name.clone(),
        IndexEntry {
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
          sha: hex::encode(sha),
          flags,
          name,
        },
      );

      // break; // until parsing done
    }

    Ok(Index {
      version,
      entries,
      changed: false,
    })
  }

  pub fn write<P>(&self, path: P) -> Result<()>
  where
    P: AsRef<Path> + fmt::Debug,
  {
    if !self.changed {
      // nothing to do!
      return Ok(());
    }

    // TODO: flock this or something
    let f = OpenOptions::new().write(true).create(true).open(path)?;
    let mut writer = BufWriter::new(f);
    let mut sha = sha1::Sha1::new();

    let mut header: Vec<u8> = Vec::with_capacity(12);
    header.extend("DIRC".as_bytes());
    header.extend(2u32.to_be_bytes().iter());
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

    writer.flush()?;

    Ok(())
  }

  pub fn add(&mut self, entry: IndexEntry) {
    self.entries.insert(entry.name.clone(), entry);
    self.changed = true;
  }

  pub fn entries(&self) -> impl Iterator<Item = &IndexEntry> {
    self.entries.values()
  }

  pub fn num_entries(&self) -> u32 {
    self.entries.len() as u32
  }
}

impl IndexEntry {
  pub fn new(path: &PathBuf) -> Result<Self> {
    use std::os::unix::fs::MetadataExt;
    let meta = path.metadata()?;

    let name = path.to_string_lossy().into_owned();
    let namelen = name.len();

    if namelen > MAX_PATH_SIZE {
      panic!("uh oh, path size is too big");
    }

    // TODO: Figure out flags; for now, I think storing just the name length is
    // sufficient. I think eventually I will want some sort of bitvector here.
    let mut flags = 0u16;
    flags = flags | (namelen as u16);

    Ok(IndexEntry {
      ctime_sec: meta.ctime() as u32,
      ctime_nano: meta.ctime_nsec() as u32,
      mtime_sec: meta.mtime() as u32,
      mtime_nano: meta.mtime_nsec() as u32,
      dev: meta.dev() as u32,
      ino: meta.ino() as u32,
      mode: meta.mode(), // XXX is this right?
      uid: meta.uid(),
      gid: meta.gid(),
      size: meta.size() as u32,
      sha: util::compute_sha_for_path(path)?.hexdigest(),
      flags,
      name,
    })
  }

  pub fn as_bytes(&self) -> Vec<u8> {
    // 64 bytes is constant, plus a filename, so allow some room for that
    let mut ret = Vec::with_capacity(100);

    // I think this is probably not very efficient.
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
    ret.extend(hex::decode(&self.sha).unwrap());
    ret.extend(self.flags.to_be_bytes().iter());
    ret.extend(self.name.as_bytes());
    ret.push(0);

    // 1-8 nul bytes as necessary to pad the entry to a multiple of eight bytes
    while ret.len() % 8 > 0 {
      ret.push(0);
    }

    ret
  }
}
