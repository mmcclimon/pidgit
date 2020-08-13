use sha1::Sha1 as Sha1Obj;
use std::fmt;

pub enum Sha {
  Object(Sha1Obj),
  Digest(String),
}

impl Sha {
  pub fn hexdigest(&self) -> String {
    match self {
      Self::Object(sha1) => sha1.hexdigest(),
      Self::Digest(s) => s.clone(),
    }
  }

  pub fn short(&self, len: usize) -> String {
    self.hexdigest()[0..len].to_string()
  }

  pub fn split_for_path(&self) -> (String, String) {
    let hex = self.hexdigest();
    let (a, b) = hex.split_at(2);
    (a.to_string(), b.to_string())
  }

  pub fn bytes(&self) -> Vec<u8> {
    // We assume here that we always have valid hex!
    match self {
      Self::Object(sha1) => sha1.digest().bytes().into(),
      Self::Digest(s) => hex::decode(s).expect("invalid hex string!"),
    }
  }
}

impl fmt::Debug for Sha {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("Sha").field(&self.hexdigest()).finish()
  }
}

impl fmt::Display for Sha {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.hexdigest())
  }
}

impl Eq for Sha {}

impl PartialEq for Sha {
  fn eq(&self, other: &Self) -> bool {
    self.hexdigest() == other.hexdigest()
  }
}

impl Clone for Sha {
  fn clone(&self) -> Self {
    match self {
      Self::Object(sha1) => Self::Object(sha1.clone()),
      Self::Digest(s) => Self::Digest(s.clone()),
    }
  }
}

impl From<Sha1Obj> for Sha {
  fn from(obj: Sha1Obj) -> Self {
    Self::Object(obj)
  }
}

impl From<String> for Sha {
  fn from(digest: String) -> Self {
    Self::Digest(digest)
  }
}

impl From<&str> for Sha {
  fn from(digest: &str) -> Self {
    Self::Digest(digest.to_string())
  }
}

impl From<[u8; sha1::DIGEST_LENGTH]> for Sha {
  fn from(bytes: [u8; sha1::DIGEST_LENGTH]) -> Self {
    Self::Digest(hex::encode(bytes))
  }
}
