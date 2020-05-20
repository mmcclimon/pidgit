use std::path::Path;

use crate::Object;

/// Given a path to an object (like .git/objects/04/2348ac8d3), this extracts
/// the 40-char sha and returns it as a string.
pub fn sha_from_path(path: &Path) -> String {
  let hunks = path
    .components()
    .map(|c| c.as_os_str().to_string_lossy())
    .collect::<Vec<_>>();

  let l = hunks.len();
  format!("{}{}", hunks[l - 2], hunks[l - 1])
}

pub fn header_for(kind: &Object, content: &[u8]) -> Vec<u8> {
  format!("{} {}\0", kind.as_str(), content.len())
    .as_bytes()
    .to_vec()
}
