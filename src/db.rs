use std::fmt;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::result::Error as QueryError;
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, HarnessWithOutput, MigrationHarness,
};
use tokio::task::block_in_place;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[rustfmt::skip]
mod schema;
pub mod autocomplete;
mod settings;
mod subscriptions;
pub mod types;

pub use settings::Settings;
pub use subscriptions::{Events, Subscription, Subscriptions, Tags};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum InitError {
    Connection(PoolError),
    Migrations(Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug)]
pub enum Error {
    Connection(PoolError),
    Query(QueryError),
}

impl std::error::Error for Error {}

pub fn init_db(database_url: &str) -> Result<DbPool, InitError> {
    block_in_place(|| {
        let mgr = ConnectionManager::new(database_url);
        let pool = Pool::new(mgr)?;

        let mut conn = pool.get()?;
        HarnessWithOutput::write_to_stdout(&mut conn)
            .run_pending_migrations(MIGRATIONS)
            .map_err(InitError::Migrations)?;

        Ok(pool)
    })
}

// impl Display & From<*> for (Init)Error {{{
impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InitError::Connection(e) => e.fmt(f),
            InitError::Migrations(e) => e.fmt(f),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Connection(e) => e.fmt(f),
            Error::Query(e) => e.fmt(f),
        }
    }
}

impl From<PoolError> for InitError {
    fn from(e: PoolError) -> InitError {
        InitError::Connection(e)
    }
}

impl From<PoolError> for Error {
    fn from(e: PoolError) -> Error {
        Error::Connection(e)
    }
}

impl From<QueryError> for Error {
    fn from(e: QueryError) -> Error {
        Error::Query(e)
    }
}
// }}}

// vim: fdm=marker
