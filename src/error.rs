use std::fmt;
use std::io::Error as IoError;

use dbl::Error as DblError;
use modio::Error as ModioError;
use pico_args::Error as ArgsError;
use prometheus::Error as PrometheusError;
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
    Twilight(TwilightError),
    Config(TomlError),
    Metrics(PrometheusError),
}

#[derive(Debug)]
pub enum DatabaseErrorInner {
    Init(DatabaseInitError),
    Query(DatabaseError),
}

#[derive(Debug)]
pub enum TwilightError {
    ClusterStart(twilight_gateway::cluster::ClusterStartError),
    Http(twilight_http::Error),
    Validation(TwilightValidation),
    Deserialization(twilight_http::response::DeserializeBodyError),
}

#[derive(Debug)]
pub enum TwilightValidation {
    Message(twilight_validate::message::MessageValidationError),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Args(e) => e.fmt(fmt),
            Error::Message(e) => e.fmt(fmt),
            Error::Io(e) => write!(fmt, "IO error: {}", e),
            Error::Twilight(TwilightError::ClusterStart(e)) => e.fmt(fmt),
            Error::Twilight(TwilightError::Http(e)) => e.fmt(fmt),
            Error::Twilight(TwilightError::Validation(TwilightValidation::Message(e))) => {
                e.fmt(fmt)
            }
            Error::Twilight(TwilightError::Deserialization(e)) => e.fmt(fmt),
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

impl From<twilight_gateway::cluster::ClusterStartError> for Error {
    fn from(e: twilight_gateway::cluster::ClusterStartError) -> Self {
        Error::Twilight(TwilightError::ClusterStart(e))
    }
}

impl From<twilight_http::Error> for Error {
    fn from(e: twilight_http::Error) -> Self {
        Error::Twilight(TwilightError::Http(e))
    }
}

impl From<twilight_http::response::DeserializeBodyError> for Error {
    fn from(e: twilight_http::response::DeserializeBodyError) -> Self {
        Error::Twilight(TwilightError::Deserialization(e))
    }
}

impl From<twilight_validate::message::MessageValidationError> for Error {
    fn from(e: twilight_validate::message::MessageValidationError) -> Self {
        Error::Twilight(TwilightError::Validation(TwilightValidation::Message(e)))
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
