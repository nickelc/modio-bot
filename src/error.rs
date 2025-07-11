use std::fmt;
use std::io::Error as IoError;

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
    Database(DatabaseErrorInner),
    Tokio(TokioError),
    Twilight(TwilightError),
    Config(TomlError),
    Metrics(PrometheusError),
}

#[derive(Debug)]
pub enum TokioError {
    Join(tokio::task::JoinError),
    WatchSend(tokio::sync::watch::error::SendError<bool>),
}

#[derive(Debug)]
pub enum DatabaseErrorInner {
    Init(DatabaseInitError),
    Query(DatabaseError),
}

#[derive(Debug)]
pub enum TwilightError {
    Start(twilight_gateway::error::StartRecommendedError),
    Http(Box<twilight_http::Error>),
    Deserialization(twilight_http::response::DeserializeBodyError),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Args(e) => e.fmt(fmt),
            Error::Message(e) => e.fmt(fmt),
            Error::Io(e) => write!(fmt, "IO error: {e}"),
            Error::Tokio(TokioError::Join(e)) => e.fmt(fmt),
            Error::Tokio(TokioError::WatchSend(e)) => e.fmt(fmt),
            Error::Twilight(TwilightError::Start(e)) => e.fmt(fmt),
            Error::Twilight(TwilightError::Http(e)) => e.fmt(fmt),
            Error::Twilight(TwilightError::Deserialization(e)) => e.fmt(fmt),
            Error::Database(DatabaseErrorInner::Init(e)) => e.fmt(fmt),
            Error::Database(DatabaseErrorInner::Query(e)) => e.fmt(fmt),
            Error::Modio(e) => e.fmt(fmt),
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

impl From<PrometheusError> for Error {
    fn from(e: PrometheusError) -> Error {
        Error::Metrics(e)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Error::Tokio(TokioError::Join(e))
    }
}

impl From<tokio::sync::watch::error::SendError<bool>> for Error {
    fn from(e: tokio::sync::watch::error::SendError<bool>) -> Self {
        Error::Tokio(TokioError::WatchSend(e))
    }
}

impl From<twilight_gateway::error::StartRecommendedError> for Error {
    fn from(e: twilight_gateway::error::StartRecommendedError) -> Self {
        Error::Twilight(TwilightError::Start(e))
    }
}

impl From<twilight_http::Error> for Error {
    fn from(e: twilight_http::Error) -> Self {
        Error::Twilight(TwilightError::Http(Box::new(e)))
    }
}

impl From<twilight_http::response::DeserializeBodyError> for Error {
    fn from(e: twilight_http::response::DeserializeBodyError) -> Self {
        Error::Twilight(TwilightError::Deserialization(e))
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
