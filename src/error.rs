use std::env::VarError;
use std::fmt;
use std::io::Error as IoError;

use serenity::Error as SerenityError;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Io(IoError),
    Serenity(SerenityError),
    Env(&'static str, VarError),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Message(e) => e.fmt(fmt),
            Error::Io(e) => write!(fmt, "IO error: {}", e),
            Error::Serenity(e) => e.fmt(fmt),
            Error::Env(key, VarError::NotPresent) => {
                write!(fmt, "Environment variable '{}' not found", key)
            }
            Error::Env(key, VarError::NotUnicode(_)) => {
                write!(fmt, "Environment variable '{}' was not valid unicode", key)
            }
        }
    }
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error::Message(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Error {
        Error::Message(s.to_string())
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Error {
        Error::Io(e)
    }
}

impl From<SerenityError> for Error {
    fn from(e: SerenityError) -> Error {
        Error::Serenity(e)
    }
}
