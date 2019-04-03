use std::collections::{HashMap, HashSet};
use std::fmt;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use log::info;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;

use crate::error::Error;
use crate::schema::settings;
use crate::util::{PoolKey, Result};

embed_migrations!("migrations");

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Default)]
pub struct Settings {
    pub game: Option<u32>,
    pub prefix: Option<String>,
}

#[derive(Insertable, AsChangeset)]
#[table_name = "settings"]
struct ChangeSettings {
    pub guild: i64,
    pub game: Option<Option<i32>>,
    pub prefix: Option<Option<String>>,
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

#[derive(Default)]
pub struct Subscriptions(pub HashMap<u32, HashSet<(ChannelId, Option<GuildId>)>>);

/// impl Display for Subscriptions {{{
impl fmt::Display for Subscriptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("{}");
        }
        f.write_str("{")?;
        let mut has_field = false;
        for (game, channels) in &self.0 {
            if has_field {
                f.write_str(", ")?;
            }
            has_field = true;
            if channels.is_empty() {
                write!(f, "{}: {{}}", game)?;
                continue;
            }
            write!(f, "{}: ", game)?;
            let mut has_subs = false;
            f.write_str("{")?;
            for (channel_id, guild_id) in channels {
                if has_subs {
                    f.write_str(", ")?;
                }
                if let Some(guild_id) = guild_id {
                    write!(f, "{}@{}", channel_id, guild_id)?;
                } else {
                    fmt::Display::fmt(&channel_id.0, f)?;
                }
                has_subs = true;
            }
            f.write_str("}")?;
        }
        f.write_str("}")
    }
}
/// }}}

impl Subscriptions {
    pub fn list_games(ctx: &mut Context, channel_id: ChannelId) -> Vec<u32> {
        let data = ctx.data.lock();
        data.get::<Subscriptions>()
            .expect("failed to get settings map")
            .0
            .iter()
            .filter_map(|(&k, v)| v.iter().find(|(chan, _)| chan == &channel_id).map(|_| k))
            .collect()
    }

    pub fn add(
        ctx: &mut Context,
        game_id: u32,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
    ) -> Result<()> {
        use crate::schema::subscriptions::dsl::*;

        {
            let mut data = ctx.data.lock();
            data.get_mut::<Subscriptions>()
                .expect("failed to get settings map")
                .0
                .entry(game_id)
                .or_insert_with(Default::default)
                .insert((channel_id, guild_id));
        }

        let data = ctx.data.lock();
        let pool = data
            .get::<PoolKey>()
            .expect("failed to get connection pool");

        pool.get()
            .map_err(Error::from)
            .and_then(|conn| {
                diesel::replace_into(subscriptions)
                    .values((
                        game.eq(game_id as i32),
                        channel.eq(channel_id.0 as i64),
                        guild.eq(guild_id.map(|g| g.0 as i64)),
                    ))
                    .execute(&conn)
                    .map_err(Error::from)
            })
            .map(|_| ())
    }

    pub fn remove(
        ctx: &mut Context,
        game_id: u32,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
    ) -> Result<()> {
        use crate::schema::subscriptions::dsl::*;

        {
            let mut data = ctx.data.lock();
            data.get_mut::<Subscriptions>()
                .expect("failed to get settings map")
                .0
                .entry(game_id)
                .or_insert_with(Default::default)
                .remove(&(channel_id, guild_id));
        }

        let data = ctx.data.lock();
        let pool = data
            .get::<PoolKey>()
            .expect("failed to get connection pool");

        pool.get()
            .map_err(Error::from)
            .and_then(|conn| {
                let pred = game.eq(game_id as i32).and(channel.eq(channel_id.0 as i64));
                let filter = subscriptions.filter(pred);
                diesel::delete(filter).execute(&conn).map_err(Error::from)
            })
            .map(|_| ())
    }
}

pub fn init_db(database_url: String) -> Result<DbPool> {
    let mgr = ConnectionManager::new(database_url);
    let pool = Pool::new(mgr)?;

    embedded_migrations::run_with_output(&pool.get()?, &mut std::io::stdout())?;

    Ok(pool)
}

pub fn load_settings(pool: &DbPool, guilds: &[GuildId]) -> Result<HashMap<GuildId, Settings>> {
    use crate::schema::settings::dsl::*;

    type Record = (i64, Option<i32>, Option<String>);

    pool.get()
        .map_err(Error::from)
        .and_then(|conn| {
            let it = guilds.iter().map(|g| g.0 as i64);
            let ids = it.collect::<Vec<_>>();
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
}

pub fn load_subscriptions(pool: &DbPool, guilds: &[GuildId]) -> Result<Subscriptions> {
    use crate::schema::subscriptions::dsl::*;
    pool.get()
        .map_err(Error::from)
        .and_then(|conn| {
            let it = guilds.iter().map(|g| g.0 as i64);
            let ids = it.collect::<Vec<_>>();
            let filter = subscriptions.filter(guild.ne_all(ids));
            match diesel::delete(filter).execute(&conn).map_err(Error::from) {
                Ok(num) => info!("Deleted {} subscription(s).", num),
                Err(e) => eprintln!("{}", e),
            }
            Ok(conn)
        })
        .and_then(|conn| {
            subscriptions
                .load::<(i32, i64, Option<i64>)>(&conn)
                .map_err(Error::from)
        })
        .and_then(|list| {
            Ok(Subscriptions(list.into_iter().fold(
                Default::default(),
                |mut map, (game_id, channel_id, guild_id)| {
                    let guild_id = guild_id.map(|id| GuildId(id as u64));
                    map.entry(game_id as u32)
                        .or_insert_with(Default::default)
                        .insert((ChannelId(channel_id as u64), guild_id));
                    map
                },
            )))
        })
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

// vim: fdm=marker
