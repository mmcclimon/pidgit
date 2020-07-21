mod wrapping_vec;

pub use wrapping_vec::WrappingVec;

use ansi_term::{ANSIGenericString, Style};
use sha1::Sha1;
use std::collections::HashSet;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

use crate::prelude::*;

pub fn find_repo() -> Option<Repository> {
  // so that PIDGIT_DIR=.git works for quick desk-checking
  if let Ok(dir) = std::env::var("PIDGIT_DIR") {
    let path = PathBuf::from(dir)
      .canonicalize()
      .expect("couldn't canonicalize PIDGIT_DIR");
    return Repository::from_git_dir(&path).map_or_else(
      |err| {
        eprintln!("{:?}", err);
        None
      },
      |repo| Some(repo),
    );
  }

  let pwd = std::env::current_dir();

  if pwd.is_err() {
    return None;
  }

  let repo = pwd
    .unwrap()
    .ancestors()
    .filter(|p| p.join(".pidgit").is_dir())
    .next()
    .map(|p| Repository::from_work_tree(p).unwrap());

  repo
}

/// Given a path to an object (like .git/objects/04/2348ac8d3), this extracts
/// the 40-char sha and returns it as a string.
pub fn sha_from_path(path: &Path) -> String {
  let hunks = path
    .components()
    .map(|c| c.as_os_str().to_str().unwrap())
    .collect::<Vec<_>>();

  let l = hunks.len();
  format!("{}{}", hunks[l - 2], hunks[l - 1])
}

// Get the sha for a file on disk, without reading the whole thing into memory.
pub fn compute_sha_for_path(
  path: &Path,
  meta: Option<&Metadata>,
) -> Result<Sha1> {
  use std::fs::File;
  use std::io::{BufRead, BufReader};

  let mut reader = BufReader::new(File::open(&path)?);
  let mut sha = Sha1::new();
  let len = match meta {
    Some(meta) => meta.len(),
    None => path.metadata()?.len(),
  };

  sha.update(format!("blob {}\0", len).as_bytes());

  loop {
    let buf = reader.fill_buf()?;
    let len = buf.len();

    // EOF
    if len == 0 {
      break;
    }

    sha.update(&buf);
    reader.consume(len);
  }

  Ok(sha)
}

fn should_color() -> bool {
  use atty::Stream;

  if cfg!(test) {
    return false;
  }

  atty::is(Stream::Stdout)
}

pub fn colored(s: &str, style: Style) -> ANSIGenericString<str> {
  if should_color() {
    style.paint(s)
  } else {
    Style::new().paint(s)
  }
}

// docs from git-check-ref-name
// 1.  They can include slash / for hierarchical (directory) grouping, but no
//     slash-separated component can begin with a dot . or end with the sequence
//     .lock.
// 2.  They must contain at least one /. This enforces the presence of a category
//     like heads/, tags/ etc. but the actual names are not restricted. If the
//     --allow-onelevel option is used, this rule is waived.
// 3.  They cannot have two consecutive dots .. anywhere.
// 4.  They cannot have ASCII control characters (i.e. bytes whose values are lower
//     than \040, or \177 DEL), space, tilde ~, caret ^, or colon : anywhere.
// 5.  They cannot have question-mark ?, asterisk *, or open bracket [ anywhere. See
//     the --refspec-pattern option below for an exception to this rule.
// 6.  They cannot begin or end with a slash / or contain multiple consecutive
//     slashes (see the --normalize option below for an exception to this rule)
// 7.  They cannot end with a dot ..
// 8.  They cannot contain a sequence @{.
// 9.  They cannot be the single character @.
// 10. They cannot contain a \.
pub fn is_valid_refname(s: &str) -> bool {
  use std::iter::FromIterator;

  if s == "@" || s.starts_with("/") || s.ends_with("/") || s.ends_with(".") {
    return false; // #6, #7, #9
  }

  let forbidden_chars: HashSet<char> =
    HashSet::from_iter(vec!['\\', ' ', '~', '^', ':', '?', '*', '[']);

  if s
    .chars()
    .any(|c| c.is_ascii_control() || forbidden_chars.contains(&c))
  {
    return false; // #4, #5, #10
  }

  let forbidden_patterns = &["..", "@{", "//"];

  for p in forbidden_patterns {
    if s.contains(p) {
      return false; // #3, #6, #8,
    }
  }

  for hunk in s.split("/") {
    if hunk.starts_with(".") || hunk.ends_with(".lock") {
      return false; // #1
    }
  }

  // I am ignoring, here, #2.
  true
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn refnames() {
    let bad = &[
      "@",
      "foo..bar",
      "../bar",
      "control\x07char",
      "poo\x7fp",
      "cool branch",
      "cool^branch",
      "cool:branch",
      "cool~branch",
      "cool?branch",
      "cool*branch",
      "cool[branch]",
      ".hidden",
      "nested/.hidden",
      "branch.lock",
      "nested/branch.lock",
      "branch.lock/other",
      "bad.",
      "some@{branch}",
      "back\\slashed",
      "/absolute",
      "dir/",
    ];

    let good = &[
      "branch",
      "branch/withslash",
      "lots-o-dashes",
      "under_scores",
      "dots.i.guess",
      "bananas-🍌",
    ];

    for name in bad {
      assert!(!is_valid_refname(name), format!("name is bad: {:?}", name));
    }

    for name in good {
      assert!(is_valid_refname(name), format!("name is ok: {:?}", name));
    }
  }
}
