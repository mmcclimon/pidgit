use std::fmt;
use std::io::Error as IoError;
use std::num::ParseIntError;
use std::str::Utf8Error;

#[derive(Debug)]
pub enum PidgitError {
  Generic(String),
  Io(IoError),
  Encoding(Utf8Error),
  Internal(Box<dyn std::error::Error>),
}

type PE = PidgitError;
pub type Result<T> = std::result::Result<T, PidgitError>;

impl std::error::Error for PidgitError {}

impl fmt::Display for PidgitError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      PE::Io(err) => write!(f, "{}", err),
      PE::Generic(err) => write!(f, "{}", err),
      PE::Encoding(err) => write!(f, "{}", err),
      PE::Internal(err) => write!(f, "weird error: {}", err),
    }
  }
}

impl From<IoError> for PidgitError {
  fn from(err: IoError) -> Self {
    PE::Io(err)
  }
}

impl From<Utf8Error> for PidgitError {
  fn from(err: Utf8Error) -> Self {
    PE::Encoding(err)
  }
}

impl From<ParseIntError> for PidgitError {
  fn from(err: ParseIntError) -> Self {
    PE::Internal(Box::new(err))
  }
}
