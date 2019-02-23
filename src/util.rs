use std::collections::HashMap;
use std::env;
use std::env::VarError;
use std::fmt;

use chrono::prelude::*;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use log::info;
use modio::auth::Credentials;
use modio::Modio;
use serenity::client::EventHandler;
use serenity::client::{Client, Context};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use tokio::runtime::Runtime;

use crate::embedded_migrations;
use crate::error::Error;
use crate::schema::settings;
use crate::{DATABASE_URL, DISCORD_BOT_TOKEN, MODIO_API_KEY, MODIO_TOKEN};
use crate::{DEFAULT_MODIO_HOST, MODIO_HOST};

pub type CliResult = std::result::Result<(), Error>;
pub type Result<T> = std::result::Result<T, Error>;
type Record = (i64, Option<i32>, Option<String>);

#[derive(Default)]
pub struct Settings {
    pub game: Option<u32>,
    pub prefix: Option<String>,
}

#[derive(Insertable, AsChangeset)]
#[table_name = "settings"]
struct ChangeSettings {
    guild: i64,
    game: Option<Option<i32>>,
    prefix: Option<Option<String>>,
}

impl From<(GuildId, u32)> for ChangeSettings {
    fn from(c: (GuildId, u32)) -> Self {
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

impl serenity::prelude::TypeMapKey for Settings {
    type Value = HashMap<GuildId, Settings>;
}

pub struct PoolKey;

impl serenity::prelude::TypeMapKey for PoolKey {
    type Value = Pool<ConnectionManager<SqliteConnection>>;
}

pub struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, ready: Ready) {
        use crate::schema::settings::dsl::*;

        let map = {
            let data = ctx.data.lock();
            let pool = data
                .get::<PoolKey>()
                .expect("failed to get connection pool");

            pool.get()
                .map_err(Error::from)
                .and_then(|conn| {
                    let it = ready.guilds.iter().map(|g| g.id().0 as i64);
                    let ids = it.collect::<Vec<_>>();
                    info!("Guilds: {:?}", ids);
                    let filter = settings.filter(guild.ne_all(ids));
                    match diesel::delete(filter).execute(&conn).map_err(Error::from) {
                        Ok(num) => info!("Deleted {} guild(s).", num),
                        Err(e) => eprintln!("{}", e),
                    }
                    Ok(conn)
                })
                .and_then(|conn| settings.load::<Record>(&conn).map_err(Error::from))
                .and_then(|list| {
                    let mut map = HashMap::new();
                    for r in list {
                        map.insert(
                            GuildId(r.0 as u64),
                            Settings {
                                game: r.1.map(|id| id as u32),
                                prefix: r.2,
                            },
                        );
                    }
                    Ok(map)
                })
                .unwrap_or_default()
        };
        let mut data = ctx.data.lock();
        data.insert::<Settings>(map);
    }
}

impl Settings {
    fn persist(ctx: &mut Context, change: ChangeSettings) {
        use crate::schema::settings::dsl::*;

        let data = ctx.data.lock();
        let pool = data
            .get::<PoolKey>()
            .expect("failed to get connection pool");

        let ret = pool.get().map_err(Error::from).and_then(|conn| {
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

    pub fn game(ctx: &mut Context, guild: GuildId) -> Option<u32> {
        let data = ctx.data.lock();
        let map = data.get::<Settings>().expect("failed to get settings map");
        map.get(&guild).and_then(|s| s.game)
    }

    pub fn set_game(ctx: &mut Context, guild: GuildId, game: u32) {
        {
            let mut data = ctx.data.lock();
            data.get_mut::<Settings>()
                .expect("failed to get settings map")
                .entry(guild)
                .or_insert_with(Default::default)
                .game = Some(game);
        }

        let change = (guild, game);
        Settings::persist(ctx, change.into());
    }

    pub fn prefix(ctx: &mut Context, msg: &Message) -> Option<String> {
        msg.guild_id.and_then(|id| {
            let data = ctx.data.lock();
            let map = data.get::<Settings>().expect("failed to get settings map");
            map.get(&id).and_then(|s| s.prefix.clone())
        })
    }

    pub fn set_prefix(ctx: &mut Context, guild: GuildId, prefix: Option<String>) {
        {
            let mut data = ctx.data.lock();
            data.get_mut::<Settings>()
                .expect("failed to get settings map")
                .entry(guild)
                .or_insert_with(Default::default)
                .prefix = prefix.clone();
        }

        let change = (guild, prefix);
        Settings::persist(ctx, change.into());
    }
}

#[derive(Debug, Clone)]
pub enum Identifier {
    Id(u32),
    Search(String),
}

// impl FromStr & Display for Identifier {{{
impl std::str::FromStr for Identifier {
    type Err = std::string::ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.parse::<u32>() {
            Ok(id) => Ok(Identifier::Id(id)),
            Err(_) => Ok(Identifier::Search(String::from(s))),
        }
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Identifier::Id(id) => id.fmt(fmt),
            Identifier::Search(id) => id.fmt(fmt),
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

fn credentials() -> Result<Credentials> {
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

pub fn initialize() -> Result<(Client, Modio, Runtime)> {
    let token = var(DISCORD_BOT_TOKEN)?;
    let database_url = var(DATABASE_URL)?;

    let mgr = ConnectionManager::new(database_url);
    let pool = Pool::new(mgr)?;

    embedded_migrations::run_with_output(&pool.get()?, &mut std::io::stdout())?;

    let client = Client::new(&token, Handler)?;
    {
        let mut data = client.data.lock();
        data.insert::<PoolKey>(pool);
    }

    let modio = {
        let host = var_or(MODIO_HOST, DEFAULT_MODIO_HOST)?;

        Modio::builder(credentials()?)
            .host(host)
            .agent("modbot")
            .build()
            .map_err(Error::from)?
    };

    let rt = Runtime::new()?;

    Ok((client, modio, rt))
}

// vim: fdm=marker
