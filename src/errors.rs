use std::fmt;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum PidgitError {
  Io(IoError),
  Generic(String),
}

type PE = PidgitError;
pub type Result<T> = std::result::Result<T, PidgitError>;

impl std::error::Error for PidgitError {}

impl fmt::Display for PidgitError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      PE::Io(err) => write!(f, "{}", err),
      PE::Generic(err) => write!(f, "{}", err),
    }
  }
}

impl From<IoError> for PidgitError {
  fn from(err: IoError) -> Self {
    PE::Io(err)
  }
}
