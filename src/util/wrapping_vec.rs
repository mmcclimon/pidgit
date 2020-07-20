#[derive(Debug, Clone)]
pub struct WrappingVec<T: Default + Clone> {
  size:    usize,
  storage: Vec<T>,
}

fn modulo(n: isize, k: usize) -> usize {
  let k = k as isize;
  (((n % k) + k) % k) as usize
}

impl<T: Default + Clone> WrappingVec<T> {
  pub fn new(size: usize) -> Self {
    Self {
      storage: vec![Default::default(); size],
      size,
    }
  }
}

impl<T: Default + Clone> std::ops::Index<isize> for WrappingVec<T> {
  type Output = T;
  fn index(&self, idx: isize) -> &Self::Output {
    &self.storage[modulo(idx, self.size)]
  }
}

impl<T: Default + Clone> std::ops::IndexMut<isize> for WrappingVec<T> {
  fn index_mut(&mut self, idx: isize) -> &mut Self::Output {
    &mut self.storage[modulo(idx, self.size)]
  }
}

impl<T: Default + Clone> std::convert::From<Vec<T>> for WrappingVec<T> {
  fn from(other: Vec<T>) -> Self {
    Self {
      size:    other.len(),
      storage: other,
    }
  }
}

impl<T: Default + Clone> WrappingVec<T> {
  pub fn len(&self) -> usize {
    self.size
  }

  pub fn get(&self, idx: usize) -> Option<&T> {
    self.storage.get(idx)
  }

  // take ownership of an element, replacing it with the default
  pub fn take(&mut self, idx: usize) -> Option<T> {
    if self.get(idx).is_some() {
      let el = std::mem::replace(&mut self.storage[idx], Default::default());
      Some(el)
    } else {
      None
    }
  }
}
