use chrono::{DateTime, FixedOffset};
use std::io::prelude::*;
use std::io::BufReader;

use crate::object::{GitObject, RawObject};
use crate::Repository;

#[derive(Debug)]
pub struct Commit {
  raw:                RawObject,
  pub tree:           String,      // sha
  pub parent_shas:    Vec<String>, // shas
  pub author:         Person,
  pub author_date:    DateTime<FixedOffset>,
  pub committer:      Person,
  pub committer_date: DateTime<FixedOffset>,
  pub message:        String,
}

// I have no idea what to call this
#[derive(Debug)]
pub struct Person {
  pub name:  String,
  pub email: String,
}

impl GitObject for Commit {
  fn get_ref(&self) -> &RawObject {
    &self.raw
  }
}

impl From<RawObject> for Commit {
  // really, this should be TryFrom, because we unwrap() a bunch of io errors in
  // here, but if we panic, it's only because there's something really weird
  // going on and we couldn't recover anyway.
  fn from(raw: RawObject) -> Self {
    use std::io::Cursor;

    // a commit has:
    // - a tree
    // - zero or more parents
    // - an author
    // - a committer
    // - (maybe, which I will ignore here...Signed-Off-By, other stuff?)
    // - a blank line
    // - a message

    let mut reader = Cursor::new(&raw.content);
    let len = reader.get_ref().len();

    let mut tree = None;
    let mut author = None;
    let mut author_date = None;
    let mut committer = None;
    let mut committer_date = None;
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
          let parsed = parse_author_line(&words[1..].join(" "));
          author = Some(parsed.0);
          author_date = Some(parsed.1);
        },
        "committer" => {
          let parsed = parse_author_line(&words[1..].join(" "));
          committer = Some(parsed.0);
          committer_date = Some(parsed.1);
        },
        "parent" => parents.push(words[1].to_string()),
        _ => break,
      }
    }

    // rest
    let mut message = String::new();
    reader.read_to_string(&mut message).unwrap();

    Self {
      raw,
      tree: tree.expect("did not find tree"),
      parent_shas: parents,
      author: author.expect("did not find author"),
      author_date: author_date.expect("did not find author"),
      committer: committer.expect("did not find committer"),
      committer_date: committer_date.expect("did not find committer date"),
      message,
    }
  }
}

fn parse_author_line(line: &str) -> (Person, DateTime<FixedOffset>) {
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

  let person = Person {
    name:  String::from_utf8(name).unwrap().trim().to_string(),
    email: String::from_utf8(email).unwrap().trim().to_string(),
  };

  (person, dt)
}

impl Commit {
  // passing the repo here is bunk
  pub fn parents(&self, repo: &Repository) -> Vec<Commit> {
    self
      .parent_shas
      .iter()
      .map(|sha| Self::from(repo.object_for_sha(sha).unwrap()))
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
