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
use serenity::model::guild::GuildStatus;
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

            let guilds = ready.guilds.iter().map(GuildStatus::id).collect::<Vec<_>>();
            info!("Guilds: {:?}", guilds);

            let settings = load_settings(&pool, &guilds).unwrap_or_default();
            let subs = load_subscriptions(&pool, &guilds).unwrap_or_default();
            info!("Subscriptions: {}", subs);

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

#[derive(Debug)]
pub struct ContentBuilder {
    limit: usize,
    pub content: Vec<String>,
}

impl ContentBuilder {
    pub fn new(limit: usize) -> Self {
        Self {
            content: vec![],
            limit,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

impl Default for ContentBuilder {
    fn default() -> Self {
        Self::new(2000)
    }
}

impl IntoIterator for ContentBuilder {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.content.into_iter()
    }
}

impl fmt::Write for ContentBuilder {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.content.last_mut() {
            Some(current) => {
                if current.len() + s.len() > self.limit {
                    self.content.push(String::from(s));
                } else {
                    current.push_str(s);
                }
            }
            None => {
                self.content.push(String::from(s));
            }
        };
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        match self.content.last_mut() {
            Some(current) => {
                if current.len() + c.len_utf8() > self.limit {
                    self.content.push(c.to_string());
                } else {
                    current.push(c);
                }
            }
            None => self.content.push(c.to_string()),
        };
        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use super::ContentBuilder;
    use std::fmt::Write;

    #[test]
    fn content_builder() {
        let mut c = ContentBuilder::new(20);

        let _ = write!(&mut c, "{}", "foo".repeat(5));
        assert_eq!(c.content.len(), 1);

        let _ = write!(&mut c, "{}", "foo".repeat(5));
        assert_eq!(c.content.len(), 2);
        assert_eq!(c.content[0], "foo".repeat(5));
        assert_eq!(c.content[1], "foo".repeat(5));

        let _ = c.write_char('f');
        let _ = c.write_char('o');
        let _ = c.write_char('o');
        assert_eq!(c.content.len(), 2);
        assert_eq!(c.content[1], "foo".repeat(6));

        let _ = c.write_str("foobar");
        assert_eq!(c.content.len(), 3);
        assert_eq!(c.content[0], "foo".repeat(5));
        assert_eq!(c.content[1], "foo".repeat(6));
        assert_eq!(c.content[2], "foobar");
    }
}

// vim: fdm=marker
