#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use std::collections::HashSet;
use std::fmt;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PoolError};
use diesel::result::Error as QueryError;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, HarnessWithOutput, MigrationHarness,
};
use tokio::task::block_in_place;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[rustfmt::skip]
mod schema;
mod settings;
mod subscriptions;

pub use settings::{load_settings, Settings};
pub use subscriptions::{Events, Subscriptions, Tags};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type GameId = u32;
pub type ChannelId = u64;
pub type GuildId = u64;
pub type UserId = u64;
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

#[derive(Default, Debug, Clone)]
pub struct Blocked {
    pub guilds: HashSet<GuildId>,
    pub users: HashSet<UserId>,
}

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

#[allow(dead_code)]
pub fn load_blocked(pool: &DbPool) -> Result<Blocked> {
    use schema::blocked_guilds::dsl::*;
    use schema::blocked_users::dsl::*;

    block_in_place(|| {
        let conn = &mut pool.get()?;
        let guilds = blocked_guilds.load::<(i64,)>(conn).ok().unwrap_or_default();
        let users = blocked_users.load::<(i64,)>(conn).ok().unwrap_or_default();
        let guilds = guilds.iter().map(|id| id.0 as GuildId).collect();
        let users = users.iter().map(|id| id.0 as UserId).collect();
        Ok(Blocked { guilds, users })
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
