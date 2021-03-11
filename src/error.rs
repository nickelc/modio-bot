use std::fmt;
use std::io::Error as IoError;

use dbl::Error as DblError;
use modio::Error as ModioError;
use pico_args::Error as ArgsError;
use prometheus::Error as PrometheusError;
use serenity::Error as SerenityError;
use toml::de::Error as TomlError;

use crate::db::Error as DatabaseError;
use crate::db::InitError as DatabaseInitError;

#[derive(Debug)]
pub enum Error {
    Args(ArgsError),
    Message(String),
    Io(IoError),
    Modio(ModioError),
    Dbl(DblError),
    Database(DatabaseErrorInner),
    Serenity(SerenityError),
    Config(TomlError),
    Metrics(PrometheusError),
}

#[derive(Debug)]
pub enum DatabaseErrorInner {
    Init(DatabaseInitError),
    Query(DatabaseError),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Args(e) => e.fmt(fmt),
            Error::Message(e) => e.fmt(fmt),
            Error::Io(e) => write!(fmt, "IO error: {}", e),
            Error::Serenity(e) => e.fmt(fmt),
            Error::Database(DatabaseErrorInner::Init(e)) => e.fmt(fmt),
            Error::Database(DatabaseErrorInner::Query(e)) => e.fmt(fmt),
            Error::Modio(e) => e.fmt(fmt),
            Error::Dbl(e) => e.fmt(fmt),
            Error::Config(e) => e.fmt(fmt),
            Error::Metrics(e) => e.fmt(fmt),
        }
    }
}

impl From<ArgsError> for Error {
    fn from(e: ArgsError) -> Error {
        Error::Args(e)
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

impl From<TomlError> for Error {
    fn from(e: TomlError) -> Error {
        Error::Config(e)
    }
}

impl From<ModioError> for Error {
    fn from(e: ModioError) -> Error {
        Error::Modio(e)
    }
}

impl From<DblError> for Error {
    fn from(e: DblError) -> Error {
        Error::Dbl(e)
    }
}

impl From<PrometheusError> for Error {
    fn from(e: PrometheusError) -> Error {
        Error::Metrics(e)
    }
}

impl From<SerenityError> for Error {
    fn from(e: SerenityError) -> Error {
        Error::Serenity(e)
    }
}

impl From<DatabaseInitError> for Error {
    fn from(e: DatabaseInitError) -> Error {
        Error::Database(DatabaseErrorInner::Init(e))
    }
}

impl From<DatabaseError> for Error {
    fn from(e: DatabaseError) -> Error {
        Error::Database(DatabaseErrorInner::Query(e))
    }
}
