use std::collections::HashMap;
use std::env;
use std::env::VarError;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::prelude::*;
use log::info;
use modio::auth::Credentials;
use modio::Modio;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use tokio::runtime::Runtime;

use crate::db::{init_db, load_settings, load_subscriptions, DbPool, Settings, Subscriptions};
use crate::error::Error;
use crate::{DATABASE_URL, DISCORD_BOT_TOKEN, MODIO_API_KEY, MODIO_TOKEN};
use crate::{DEFAULT_MODIO_HOST, MODIO_HOST};

pub type CliResult = std::result::Result<(), Error>;
pub type Result<T> = std::result::Result<T, Error>;

impl TypeMapKey for Settings {
    type Value = HashMap<GuildId, Settings>;
}

impl TypeMapKey for Subscriptions {
    type Value = Subscriptions;
}

pub struct PoolKey;

impl TypeMapKey for PoolKey {
    type Value = DbPool;
}

pub struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, ready: Ready) {
        let (settings, subs) = {
            let data = ctx.data.lock();
            let pool = data
                .get::<PoolKey>()
                .expect("failed to get connection pool");

            let guilds = ready.guilds.iter().map(|g| g.id()).collect::<Vec<_>>();
            info!("Guilds: {:?}", guilds);

            let settings = load_settings(&pool, &guilds).unwrap_or_default();
            let subs = load_subscriptions(&pool).unwrap_or_default();
            info!("Subscriptions: {:?}", subs.0);

            (settings, subs)
        };
        let mut data = ctx.data.lock();
        data.insert::<Settings>(settings);
        data.insert::<Subscriptions>(subs);
    }
}

pub fn dynamic_prefix(ctx: &mut Context, msg: &Message) -> Option<String> {
    Settings::prefix(ctx, msg)
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

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

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

    let pool = init_db(database_url)?;

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
