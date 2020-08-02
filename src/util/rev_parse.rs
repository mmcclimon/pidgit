use std::collections::HashSet;

use crate::object::Object;
use crate::prelude::*;

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

// silly
fn is_valid_refname_allow_at(s: &str) -> bool {
  if s == "@" {
    true
  } else {
    is_valid_refname(s)
  }
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq)]
enum Revision {
  Ref(String),
  Parent(Box<Revision>),
  Ancestor(Box<Revision>, u32),
}

#[allow(unused)]
// Parse a revision into an AST (a Revision enum). These boxes are kind of
// annoying, but such is life.
fn parse_rev(revision: &str) -> Option<Revision> {
  if revision.ends_with('^') {
    let rev = parse_rev(&revision[0..revision.len() - 1]);
    rev.map(|parent| Revision::Parent(Box::new(parent)))
  } else if revision.contains('~') {
    // regex would really be better here
    let hunks = revision.split('~').take(2).collect::<Vec<_>>();
    let n = if let Ok(num) = hunks[1].parse::<u32>() {
      num
    } else {
      return None;
    };

    parse_rev(hunks[0]).map(|ancestor| Revision::Ancestor(Box::new(ancestor), n))
  } else if is_valid_refname_allow_at(revision) {
    let name = if revision == "@" { "HEAD" } else { revision };
    Some(Revision::Ref(name.to_string()))
  } else {
    None
  }
}

fn resolve_rev(revision: &Revision, repo: &Repository) -> Option<Object> {
  match revision {
    Revision::Ref(refname) => repo
      .resolve_ref(refname)
      .or_else(|_| repo.resolve_sha(refname))
      .ok()
      .and_then(|obj| match obj {
        Object::Commit(_) => Some(obj),
        _ => None,
      }),
    Revision::Parent(rev) => resolve_rev(rev, repo).and_then(|obj| match obj {
      Object::Commit(commit) => commit.parent(repo).map(|c| Object::Commit(c)),
      _ => None,
    }),
    Revision::Ancestor(ref rev, mut n) => {
      resolve_rev(rev, repo).and_then(|obj| {
        let mut commit = match obj {
          Object::Commit(commit) => Some(commit),
          _ => None,
        };

        while commit.is_some() && n > 0 {
          commit = commit.unwrap().parent(repo);
          n -= 1;
        }

        commit.map(|c| Object::Commit(c))
      })
    },
  }
}

// This is here, rather than in the repo impl, so that we don't have to leak the
// Revision enum, which isn't generally useful elsewhere.
pub fn resolve_revision(revstr: &str, repo: &Repository) -> Option<Object> {
  parse_rev(revstr).and_then(|rev| resolve_rev(&rev, repo))
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

  #[test]
  fn parse() {
    use Revision::*;
    assert_eq!(
      parse_rev("master^"),
      Some(Parent(Box::new(Ref("master".into()))))
    );

    assert_eq!(parse_rev("@^"), Some(Parent(Box::new(Ref("HEAD".into())))));

    assert_eq!(
      parse_rev("HEAD~42"),
      Some(Ancestor(Box::new(Ref("HEAD".into())), 42))
    );

    #[rustfmt::skip]
    assert_eq!(
      parse_rev("master^^"),
      Some(Parent(Box::new(Parent(Box::new(Ref("master".into()))))))
    );

    assert_eq!(
      parse_rev("abc123~3^"),
      Some(Parent(Box::new(Ancestor(
        Box::new(Ref("abc123".into())),
        3
      ))))
    );

    assert_eq!(parse_rev("/../foo^"), None);
    assert_eq!(parse_rev("apple:pie~3"), None);
    assert_eq!(parse_rev("foo~banana"), None);
  }
}
