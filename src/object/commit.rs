use chrono::{DateTime, FixedOffset};
use std::fmt;
use std::io::prelude::*;
use std::io::BufReader;

use crate::object::Object;
use crate::prelude::*;

#[derive(Debug)]
pub struct Commit {
  pub tree:        String,      // sha
  pub parent_shas: Vec<String>, // shas
  pub author:      Person,
  pub committer:   Person,
  pub message:     String,
  pub content:     Option<Vec<u8>>,
}

// I have no idea what to call this
#[derive(Debug, Clone)]
pub struct Person {
  pub name:  String,
  pub email: String,
  pub date:  DateTime<FixedOffset>,
}

impl GitObject for Commit {
  fn raw_content(&self) -> Vec<u8> {
    if let Some(c) = &self.content {
      return c.clone();
    }

    let mut lines = vec![];

    lines.push(format!("tree {}", self.tree));

    for parent in &self.parent_shas {
      lines.push(format!("parent {}", parent));
    }

    lines.push(format!(
      "author {} {}",
      self.author,
      self.author.date.format("%s %z")
    ));

    lines.push(format!(
      "committer {} {}",
      self.committer,
      self.committer.date.format("%s %z")
    ));

    lines.push("".to_string());

    lines.push(format!("{}", self.message));

    lines.join("\n").as_bytes().to_vec()
  }

  fn type_str(&self) -> &str {
    "commit"
  }
}

impl Commit {
  // really, this should be TryFrom, because we unwrap() a bunch of io errors in
  // here, but if we panic, it's only because there's something really weird
  // going on and we couldn't recover anyway.
  pub fn from_content(content: Vec<u8>) -> Self {
    use std::io::Cursor;

    // a commit has:
    // - a tree
    // - zero or more parents
    // - an author
    // - a committer
    // - (maybe, which I will ignore here...Signed-Off-By, other stuff?)
    // - a blank line
    // - a message

    let mut reader = Cursor::new(&content);
    let len = reader.get_ref().len();

    let mut tree = None;
    let mut author = None;
    let mut committer = None;
    let mut parents = vec![];

    while (reader.position() as usize) < len {
      let mut line = String::new();
      reader.read_line(&mut line).unwrap();
      line.pop();

      let words = line.split(" ").collect::<Vec<_>>();

      // empty string
      if words.len() == 1 {
        break;
      }

      match words[0] {
        "tree" => tree = Some(words[1].to_string()),
        "author" => {
          author = Some(parse_author_line(&words[1..].join(" ")));
        },
        "committer" => {
          committer = Some(parse_author_line(&words[1..].join(" ")));
        },
        "parent" => parents.push(words[1].to_string()),
        _ => break,
      }
    }

    // rest
    let mut message = String::new();
    reader.read_to_string(&mut message).unwrap();

    Self {
      tree: tree.expect("did not find tree"),
      parent_shas: parents,
      author: author.expect("did not find author"),
      committer: committer.expect("did not find committer"),
      message,
      content: Some(content),
    }
  }
}

fn parse_author_line(line: &str) -> Person {
  // probably there's a better way to do this...
  let mut reader = BufReader::new(line.as_bytes());

  let mut name = vec![];
  reader.read_until(b'<', &mut name).unwrap();
  name.pop();

  let mut email = vec![];
  reader.read_until(b'>', &mut email).unwrap();
  email.pop();

  let mut date = String::new();
  reader.read_to_string(&mut date).unwrap();

  let dt = DateTime::parse_from_str(date.trim(), "%s %z").unwrap();

  Person {
    name:  String::from_utf8(name).unwrap().trim().to_string(),
    email: String::from_utf8(email).unwrap().trim().to_string(),
    date:  dt,
  }
}

impl Commit {
  // passing the repo here is bunk
  pub fn parents(&self, repo: &Repository) -> Vec<Commit> {
    self
      .parent_shas
      .iter()
      .filter_map(|sha| {
        if let Object::Commit(c) = repo.object_for_sha(sha).unwrap() {
          Some(c)
        } else {
          None
        }
      })
      .collect()
  }

  pub fn title(&self) -> &str {
    let idx = self
      .message
      .find('\n')
      .or_else(|| Some(self.message.len() as usize))
      .unwrap();

    &self.message[0..idx]
  }
}

impl fmt::Display for Person {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} <{}>", self.name, self.email)
  }
}
