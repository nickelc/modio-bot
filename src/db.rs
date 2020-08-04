use std::collections::HashMap;
use std::collections::HashSet;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use log::info;
use serenity::model::channel::Message;
use serenity::model::id::GuildId;
use serenity::model::id::UserId;

use crate::error::Error;
use crate::util::Result;

embed_migrations!("migrations");

#[rustfmt::skip]
mod schema;
mod subscriptions;

use schema::settings;
pub use subscriptions::{Events, Subscriptions, Tags};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type GameId = u32;

#[derive(Default, Debug, Clone)]
pub struct Blocked {
    pub guilds: HashSet<GuildId>,
    pub users: HashSet<UserId>,
}

#[derive(Default)]
pub struct GuildSettings {
    game: Option<GameId>,
    prefix: Option<String>,
}

#[derive(Insertable, AsChangeset)]
#[table_name = "settings"]
#[allow(clippy::option_option)]
struct ChangeSettings {
    pub guild: i64,
    pub game: Option<Option<i32>>,
    pub prefix: Option<Option<String>>,
}

pub struct Settings {
    pub pool: DbPool,
    pub data: HashMap<GuildId, GuildSettings>,
}

impl Settings {
    fn persist(&self, change: ChangeSettings) {
        use schema::settings::dsl::*;

        let ret = self.pool.get().map_err(Error::from).and_then(|conn| {
            let target = settings.filter(guild.eq(change.guild));
            let query = diesel::update(target).set(&change);

            match query.execute(&conn).map_err(Error::from) {
                Ok(0) => {
                    let query = diesel::insert_into(settings).values(&change);
                    let ret = query.execute(&conn).map_err(Error::from);

                    if let Err(e) = ret {
                        eprintln!("{}", e);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
            Ok(())
        });

        if let Err(e) = ret {
            eprintln!("{}", e);
        }
    }

    pub fn game(&self, guild: GuildId) -> Option<GameId> {
        self.data.get(&guild).and_then(|s| s.game)
    }

    pub fn set_game(&mut self, guild: GuildId, game: GameId) {
        self.data.entry(guild).or_default().game = Some(game);

        let change = (guild, game);
        self.persist(change.into());
    }

    pub fn prefix(&self, msg: &Message) -> Option<String> {
        msg.guild_id
            .and_then(|id| self.data.get(&id).and_then(|s| s.prefix.clone()))
    }

    pub fn set_prefix(&mut self, guild: GuildId, prefix: Option<String>) {
        self.data.entry(guild).or_default().prefix = prefix.clone();

        let change = (guild, prefix);
        self.persist(change.into());
    }
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

pub fn load_settings(pool: &DbPool, guilds: &[GuildId]) -> Result<HashMap<GuildId, GuildSettings>> {
    use schema::settings::dsl::*;

    type Record = (i64, Option<i32>, Option<String>);

    let conn = pool.get()?;

    let it = guilds.iter().map(|g| g.0 as i64);
    let ids = it.collect::<Vec<_>>();
    let filter = settings.filter(guild.ne_all(ids));
    match diesel::delete(filter).execute(&conn).map_err(Error::from) {
        Ok(num) => info!("Deleted {} guild(s).", num),
        Err(e) => eprintln!("{}", e),
    }

    let list = settings.load::<Record>(&conn).unwrap_or_default();
    let mut map = HashMap::new();
    for r in list {
        map.insert(
            GuildId(r.0 as u64),
            GuildSettings {
                game: r.1.map(|id| id as u32),
                prefix: r.2,
            },
        );
    }
    Ok(map)
}

impl From<(GuildId, GameId)> for ChangeSettings {
    fn from(c: (GuildId, GameId)) -> Self {
        Self {
            guild: (c.0).0 as i64,
            game: Some(Some(c.1 as i32)),
            prefix: None,
        }
    }
}

impl From<(GuildId, Option<String>)> for ChangeSettings {
    fn from(c: (GuildId, Option<String>)) -> Self {
        Self {
            guild: (c.0).0 as i64,
            game: None,
            prefix: Some(c.1),
        }
    }
}

// vim: fdm=marker
