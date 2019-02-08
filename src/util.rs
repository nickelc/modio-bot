use std::collections::HashMap;
use std::env;
use std::env::VarError;
use std::fmt;
use std::io::Error as IoError;

use chrono::prelude::*;
use modio::auth::Credentials;
use serenity::client::EventHandler;
use serenity::model::id::GuildId;
use serenity::Error as SerenityError;

use crate::{MODIO_API_KEY, MODIO_TOKEN};

pub type CliResult = std::result::Result<(), Error>;
pub type Result<T> = std::result::Result<T, Error>;

pub struct Handler;
pub struct GameKey;

impl EventHandler for Handler {}

impl typemap::Key for GameKey {
    type Value = HashMap<GuildId, Identifier>;
}

#[derive(Debug, Clone)]
pub enum Identifier {
    Id(u32),
    NameId(String),
}

// impl FromStr & Display for Identifier {{{
impl std::str::FromStr for Identifier {
    type Err = std::string::ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.parse::<u32>() {
            Ok(id) => Ok(Identifier::Id(id)),
            Err(_) => Ok(Identifier::NameId(String::from(s))),
        }
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identifier::Id(id) => id.fmt(fmt),
            Identifier::NameId(id) => id.fmt(fmt),
        }
    }
}
// }}}

pub fn format_timestamp(seconds: i64) -> impl fmt::Display {
    NaiveDateTime::from_timestamp(seconds, 0).format("%Y-%m-%d %H:%M:%S")
}

pub fn var(key: &'static str) -> Result<String> {
    env::var(key).map_err(|e| Error::Env(key, e))
}

pub fn var_or<S: Into<String>>(key: &'static str, default: S) -> Result<String> {
    match env::var(key) {
        Ok(v) => Ok(v),
        Err(VarError::NotPresent) => Ok(default.into()),
        Err(e) => Err(Error::Env(key, e)),
    }
}

pub fn credentials() -> Result<Credentials> {
    use VarError::*;

    let api_key = env::var(MODIO_API_KEY);
    let token = env::var(MODIO_TOKEN);

    match (api_key, token) {
        (Ok(key), _) => Ok(Credentials::ApiKey(key)),
        (_, Ok(token)) => Ok(Credentials::Token(token)),
        (Err(NotUnicode(_)), Err(_)) => {
            Err("Environment variable 'MODIO_API_KEY' is not valid unicode".into())
        }
        (Err(_), Err(NotUnicode(_))) => {
            Err("Environment variable 'MODIO_TOKEN' is not valid unicode".into())
        }
        (Err(NotPresent), Err(NotPresent)) => {
            Err("Environment variable 'MODIO_API_KEY' or 'MODIO_TOKEN' not found".into())
        }
    }
}

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

// vim: fdm=marker
