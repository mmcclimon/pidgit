use std::ffi::OsString;
use std::fmt;
use std::io::Error as IoError;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum PidgitError {
  Generic(String),
  Clap(clap::Error),
  Io(IoError),
  Encoding(Box<dyn std::error::Error>),
  Internal(Box<dyn std::error::Error>),
  ObjectNotFound(String),
  RefNotFound(String),
  InvalidObject(&'static str), // wanted type
  InvalidRefName(String),
  PathspecNotFound(OsString),
  Index(String),
  Lock(PathBuf, IoError),
}

type PE = PidgitError;
pub type Result<T> = std::result::Result<T, PidgitError>;

impl std::error::Error for PidgitError {}

impl fmt::Display for PidgitError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      PE::Generic(err) => write!(f, "{}", err),
      PE::Clap(err) => write!(f, "{:?}", err),
      PE::Io(err) => write!(f, "{:?}", err),
      PE::Encoding(err) => write!(f, "{}", err),
      PE::Internal(err) => write!(f, "weird error: {}", err),
      PE::ObjectNotFound(sha) => write!(f, "object not found: {}", sha),
      PE::RefNotFound(refname) => write!(f, "ref not found: {}", refname),
      PE::InvalidObject(want) => write!(f, "invalid object type: not a {}", want),
      PE::InvalidRefName(name) => write!(f, "invalid ref name: {}", name),
      PE::Index(err) => write!(f, "could not parse index file: {}", err),
      PE::PathspecNotFound(spec) => {
        write!(f, "pathspec {:?} did not match any files", spec)
      },
      PE::Lock(path, err) => write!(f, "could not lock {:?}: {}", path, err),
    }
  }
}

impl From<clap::Error> for PidgitError {
  fn from(err: clap::Error) -> Self {
    PE::Clap(err)
  }
}

impl From<IoError> for PidgitError {
  fn from(err: IoError) -> Self {
    PE::Io(err)
  }
}

impl From<Utf8Error> for PidgitError {
  fn from(err: Utf8Error) -> Self {
    PE::Encoding(Box::new(err))
  }
}

impl From<FromUtf8Error> for PidgitError {
  fn from(err: FromUtf8Error) -> Self {
    PE::Encoding(Box::new(err))
  }
}

impl From<ParseIntError> for PidgitError {
  fn from(err: ParseIntError) -> Self {
    PE::Internal(Box::new(err))
  }
}
