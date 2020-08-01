use std::collections::HashSet;

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
