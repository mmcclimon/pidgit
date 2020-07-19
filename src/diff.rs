use crate::util::{colored, WrappingVec};
use std::default::Default;

const HUNK_CONTEXT: isize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiffType {
  Ins,
  Del,
  Eql,
}

#[derive(Debug)]
struct Trace(usize, usize, usize, usize);

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

#[derive(Debug)]
pub struct Myers {
  a: WrappingVec<Line>,
  b: WrappingVec<Line>,
}

impl Default for Line {
  fn default() -> Self {
    Line(0, "".into())
  }
}

pub fn diff_hunks(a: String, b: String) -> Vec<DiffHunk> {
  let differ = Myers::new(a, b);
  DiffHunk::filter(differ.diff())
}

#[rustfmt::skip]
impl Trace {
  fn new<T>(prev_x: T, prev_y: T, x: T, y: T) -> Self
  where
    T: std::convert::TryInto<usize>,
    T::Error: std::fmt::Debug,
  {
    // TODO: don't unwrap
    Self(
      prev_x.try_into().unwrap(),
      prev_y.try_into().unwrap(),
      x.try_into().unwrap(),
      y.try_into().unwrap(),
    )
  }

  fn prev_x(&self) -> usize { self.0 }
  fn prev_y(&self) -> usize { self.1 }
  fn x(&self) -> usize { self.2 }
  fn y(&self) -> usize { self.3 }
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

impl Myers {
  pub fn new(a: String, b: String) -> Self {
    Self {
      a: a
        .lines()
        .enumerate()
        .map(|(n, s)| Line(n + 1, s.to_string()))
        .collect::<Vec<_>>()
        .into(),
      b: b
        .lines()
        .enumerate()
        .map(|(n, s)| Line(n + 1, s.to_string()))
        .collect::<Vec<_>>()
        .into(),
    }
  }

  pub fn diff(&self) -> Vec<Edit> {
    let mut diff = vec![];
    for trace in self.backtrack() {
      let a_line = &self.a.get(trace.prev_x());
      let b_line = &self.b.get(trace.prev_y());

      if trace.x() == trace.prev_x() {
        diff.push(Edit::new(
          DiffType::Ins,
          None,
          Some(b_line.unwrap().clone()),
        ))
      } else if trace.y() == trace.prev_y() {
        diff.push(Edit::new(
          DiffType::Del,
          Some(a_line.unwrap().clone()),
          None,
        ))
      } else {
        diff.push(Edit::new(
          DiffType::Eql,
          Some(a_line.unwrap().clone()),
          Some(b_line.unwrap().clone()),
        ))
      }
    }

    diff.reverse();
    diff
  }

  fn shortest_edit(&self) -> Vec<WrappingVec<isize>> {
    let n = self.a.len() as isize;
    let m = self.b.len() as isize;
    let max = m + n;

    let mut v: WrappingVec<isize> = WrappingVec::new(2 * max as usize + 1);
    let mut trace = vec![];

    v[1] = 0;

    for d in 0..=max {
      trace.push(v.clone());

      for k in (-d..=d).step_by(2) {
        let mut x = if k == -d || (k != d && v[k - 1] < v[k + 1]) {
          v[k + 1]
        } else {
          v[k - 1] + 1
        };

        let mut y = x - k;

        while x < n && y < m && self.a[x].1 == self.b[y].1 {
          x += 1;
          y += 1;
        }

        v[k] = x;

        if x >= n && y >= m {
          return trace;
        }
      }
    }

    unreachable!("diff did not exit?");
  }

  fn backtrack(&self) -> Vec<Trace> {
    let mut x = self.a.len() as isize;
    let mut y = self.b.len() as isize;

    // TODO return an iterator
    let mut ret = vec![];

    for (d, v) in self.shortest_edit().iter().enumerate().rev() {
      let d = d as isize;
      let k = x - y;

      let prev_k = if k == -d || (k != d && v[k - 1] < v[k + 1]) {
        k + 1
      } else {
        k - 1
      };

      let prev_x = v[prev_k];
      let prev_y = prev_x - prev_k;

      while x > prev_x && y > prev_y {
        ret.push(Trace::new(x - 1, y - 1, x, y));
        x -= 1;
        y -= 1;
      }

      if d > 0 {
        ret.push(Trace::new(prev_x, prev_y, x, y));
      }

      x = prev_x;
      y = prev_y;
    }

    ret
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
      if offset >= 0 && counter > 0 {
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn basic() {
    // this isn't really a test, but is useful for playing around
    let a = "ABCABBA".split("").skip(1).collect::<Vec<_>>().join("\n");
    let b = "CBABAC".split("").skip(1).collect::<Vec<_>>().join("\n");

    let m = Myers::new(a, b);
    let diff = m.diff();
    for line in diff {
      println!("{}", line);
    }
  }
}
