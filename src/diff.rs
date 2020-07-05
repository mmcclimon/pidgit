use crate::util::WrappingVec;

#[derive(Debug)]
enum DiffType {
  Ins,
  Del,
  Eql,
}

#[derive(Debug)]
struct Trace(usize, usize, usize, usize);

#[derive(Debug)]
pub struct Edit<'d>(DiffType, &'d str);

#[derive(Debug)]
pub struct Myers {
  a: WrappingVec<String>,
  b: WrappingVec<String>,
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

impl<'d> std::fmt::Display for Edit<'d> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}{}", self.0, self.1)
  }
}

#[allow(unused)]
impl Myers {
  pub fn new(a: String, b: String) -> Self {
    Self {
      a: a.lines().map(|s| s.to_string()).collect::<Vec<_>>().into(),
      b: b.lines().map(|s| s.to_string()).collect::<Vec<_>>().into(),
    }
  }

  pub fn diff(&self) -> Vec<Edit> {
    let mut diff = vec![];
    for trace in self.backtrack() {
      let a_line = &self.a.get(trace.prev_x());
      let b_line = &self.b.get(trace.prev_y());

      if trace.x() == trace.prev_x() {
        diff.push(Edit(DiffType::Ins, b_line.unwrap()))
      } else if trace.y() == trace.prev_y() {
        diff.push(Edit(DiffType::Del, a_line.unwrap()))
      } else {
        diff.push(Edit(DiffType::Eql, a_line.unwrap()))
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

        while x < n && y < m && self.a[x] == self.b[y] {
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
