use std::collections::HashSet;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use serenity::model::id::GuildId;
use serenity::model::id::UserId;

use crate::util::Result;

embed_migrations!("migrations");

#[rustfmt::skip]
mod schema;
mod settings;
mod subscriptions;

pub use settings::{load_settings, Settings};
pub use subscriptions::{Events, Subscriptions, Tags};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type GameId = u32;

#[derive(Default, Debug, Clone)]
pub struct Blocked {
    pub guilds: HashSet<GuildId>,
    pub users: HashSet<UserId>,
}

pub fn init_db(database_url: &str) -> Result<DbPool> {
    let mgr = ConnectionManager::new(database_url);
    let pool = Pool::new(mgr)?;

    embedded_migrations::run_with_output(&pool.get()?, &mut std::io::stdout())?;

    Ok(pool)
}

pub fn load_blocked(pool: &DbPool) -> Result<Blocked> {
    use schema::blocked_guilds::dsl::*;
    use schema::blocked_users::dsl::*;

    let conn = pool.get()?;
    let guilds = blocked_guilds
        .load::<(i64,)>(&conn)
        .ok()
        .unwrap_or_default();
    let users = blocked_users.load::<(i64,)>(&conn).ok().unwrap_or_default();
    let guilds = guilds.iter().map(|id| GuildId(id.0 as u64)).collect();
    let users = users.iter().map(|id| UserId(id.0 as u64)).collect();
    Ok(Blocked { guilds, users })
}

// vim: fdm=marker
