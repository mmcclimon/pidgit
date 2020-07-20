mod myers;

use crate::util::colored;
use std::default::Default;

const HUNK_CONTEXT: isize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiffType {
  Ins,
  Del,
  Eql,
}

#[derive(Debug, Clone)]
struct Line(usize, String);

#[derive(Debug, Clone)]
pub struct Edit {
  kind: DiffType,
  a:    Option<Line>,
  b:    Option<Line>,
}

#[derive(Debug)]
pub struct DiffHunk {
  a_start:   usize,
  b_start:   usize,
  pub edits: Vec<Edit>,
}

impl Default for Line {
  fn default() -> Self {
    Line(0, "".into())
  }
}

pub fn diff_hunks(a: String, b: String) -> Vec<DiffHunk> {
  let differ = myers::Myers::new(a, b);
  DiffHunk::filter(differ.diff())
}

impl std::fmt::Display for DiffType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let c = match self {
      Self::Ins => '+',
      Self::Del => '-',
      Self::Eql => ' ',
    };
    write!(f, "{}", c)
  }
}

impl std::fmt::Display for Line {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.1)
  }
}

impl std::fmt::Display for Edit {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use ansi_term::{Color, Style};

    let text = if self.a.is_some() {
      self.a.as_ref()
    } else if self.b.is_some() {
      self.b.as_ref()
    } else {
      panic!("nonsensical edit")
    };

    let s = match self.kind {
      DiffType::Eql => Style::new(),
      DiffType::Ins => Color::Green.normal(),
      DiffType::Del => Color::Red.normal(),
    };

    let line = format!("{}{}", self.kind, text.unwrap());
    write!(f, "{}", colored(&line, s))
  }
}

impl DiffHunk {
  pub fn filter(diff: Vec<Edit>) -> Vec<Self> {
    let mut hunks = vec![];
    let mut offset: isize = 0;

    loop {
      while offset < diff.len() as isize
        && diff[offset as usize].kind == DiffType::Eql
      {
        offset += 1;
      }

      if offset >= diff.len() as isize {
        return hunks;
      }

      offset -= HUNK_CONTEXT + 1;

      let a_start = if offset < 0 {
        0
      } else {
        diff[offset as usize].a.as_ref().unwrap().number()
      };

      let b_start = if offset < 0 {
        0
      } else {
        diff[offset as usize].b.as_ref().unwrap().number()
      };

      let mut hunk = DiffHunk {
        a_start,
        b_start,
        edits: vec![],
      };

      offset = hunk.build(&diff, offset);

      hunks.push(hunk);
    }
  }

  fn build(&mut self, diff: &Vec<Edit>, mut offset: isize) -> isize {
    let mut counter = -1;

    while counter != 0 {
      if offset >= 0 && counter > 0 && offset < diff.len() as isize {
        self.edits.push(diff[offset as usize].clone());
      }

      offset += 1;
      if offset > diff.len() as isize {
        break;
      }

      let idx = offset + HUNK_CONTEXT;
      if idx >= diff.len() as isize {
        counter -= 1;
        continue;
      }

      match diff[idx as usize].kind {
        DiffType::Eql => counter -= 1,
        _ => counter = 2 * HUNK_CONTEXT + 1,
      }
    }

    offset
  }

  pub fn header(&self) -> String {
    let a_offset = self.offsets_for(|e| e.a.as_ref(), self.a_start);
    let b_offset = self.offsets_for(|e| e.b.as_ref(), self.b_start);

    format!(
      "@@ -{},{} +{},{} @@",
      a_offset.0, a_offset.1, b_offset.0, b_offset.1
    )
  }

  fn offsets_for<F>(&self, getter: F, default: usize) -> (usize, usize)
  where
    F: FnMut(&Edit) -> Option<&Line>,
  {
    let lines = self.edits.iter().filter_map(getter).collect::<Vec<_>>();
    let start = if lines.len() > 0 {
      lines[0].number()
    } else {
      default
    };
    (start, lines.len())
  }
}

impl Edit {
  fn new(kind: DiffType, a: Option<Line>, b: Option<Line>) -> Self {
    Edit { kind, a, b }
  }
}

impl Line {
  fn number(&self) -> usize {
    self.0
  }
}
