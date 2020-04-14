use std::collections::HashMap;
use std::collections::HashSet;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use log::info;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use serenity::model::id::UserId;

use crate::error::Error;
use crate::schema::settings;
use crate::util::Result;

embed_migrations!("migrations");

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type GameId = u32;
pub type ExcludeMods = HashSet<u32>;
pub type Subscription = (ChannelId, Option<GuildId>, Events, ExcludeMods);

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
        use crate::schema::settings::dsl::*;

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

pub struct Subscriptions {
    pub pool: DbPool,
}

bitflags::bitflags! {
    pub struct Events: i32 {
        const NEW = 0b0001;
        const UPD = 0b0010;
        const ALL = Self::NEW.bits | Self::UPD.bits;
    }
}

impl Subscriptions {
    pub fn cleanup(&self, guilds: &[GuildId]) -> Result<()> {
        use crate::schema::subscriptions::dsl::*;

        let conn = self.pool.get()?;
        let it = guilds.iter().map(|g| g.0 as i64);
        let ids = it.collect::<Vec<_>>();
        let filter = subscriptions.filter(guild.ne_all(&ids));
        let num = diesel::delete(filter).execute(&conn)?;
        info!("Deleted {} subscription(s).", num);

        {
            use crate::schema::subscriptions_exclude_mods::dsl::*;
            let filter = subscriptions_exclude_mods.filter(guild.ne_all(ids));
            let num = diesel::delete(filter).execute(&conn)?;
            info!("Deleted {} excluded mods.", num);
        }
        Ok(())
    }

    pub fn load(&self) -> Result<HashMap<GameId, Vec<Subscription>>> {
        use crate::schema::subscriptions::dsl::*;

        type Record = (i32, i64, Option<i64>, i32);

        let conn = self.pool.get()?;
        let list = subscriptions.load::<Record>(&conn)?;

        let mut excluded = self.load_excluded_mods()?;

        Ok(list.into_iter().fold(
            HashMap::new(),
            |mut map, (game_id, channel_id, guild_id, evt)| {
                let game_id = game_id as GameId;
                let channel_id = ChannelId(channel_id as u64);
                let guild_id = guild_id.map(|id| GuildId(id as u64));
                let evt = Events::from_bits_truncate(evt);
                let excluded = excluded.remove(&(game_id, channel_id)).unwrap_or_default();
                map.entry(game_id)
                    .or_default()
                    .push((channel_id, guild_id, evt, excluded));
                map
            },
        ))
    }

    fn load_excluded_mods(&self) -> Result<HashMap<(GameId, ChannelId), ExcludeMods>> {
        use crate::schema::subscriptions_exclude_mods::dsl::*;

        type Record = (i32, i64, Option<i64>, i32);

        let conn = self.pool.get()?;
        let list = subscriptions_exclude_mods.load::<Record>(&conn)?;
        Ok(list
            .into_iter()
            .fold(HashMap::new(), |mut map, (game_id, channel_id, _, mid)| {
                let key = (game_id as GameId, ChannelId(channel_id as u64));
                map.entry(key).or_default().insert(mid as u32);
                map
            }))
    }

    pub fn list_games(&self, channel_id: ChannelId) -> Result<HashMap<GameId, Events>> {
        use crate::schema::subscriptions::dsl::*;

        let conn = self.pool.get()?;

        let records = subscriptions
            .select((game, events))
            .filter(channel.eq(channel_id.0 as i64))
            .load::<(i32, i32)>(&conn)?;

        let records = records
            .into_iter()
            .map(|(game_id, evts)| (game_id as u32, Events::from_bits_truncate(evts)))
            .collect();

        Ok(records)
    }

    pub fn list_excluded(&self, channel_id: ChannelId) -> Result<HashMap<GameId, ExcludeMods>> {
        use crate::schema::subscriptions_exclude_mods::dsl::*;

        let conn = self.pool.get()?;

        let records = subscriptions_exclude_mods
            .select((game, mod_id))
            .filter(channel.eq(channel_id.0 as i64))
            .load::<(i32, i32)>(&conn)?;

        let records: HashMap<GameId, ExcludeMods> =
            records
                .into_iter()
                .fold(HashMap::new(), |mut map, (game_id, mid)| {
                    map.entry(game_id as GameId).or_default().insert(mid as u32);
                    map
                });
        Ok(records)
    }

    pub fn add(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
        evts: Events,
    ) -> Result<()> {
        use crate::schema::subscriptions::dsl::*;

        type Record = (i32, i64, Option<i64>, i32);

        let conn = self.pool.get()?;

        let pk = (game_id as i32, channel_id.0 as i64);
        let first = subscriptions.find(pk).first::<Record>(&conn);

        let (game_id, channel_id, guild_id, evts) = match first {
            Ok((game_id, channel_id, guild_id, old_evts)) => {
                let mut new_evts = Events::from_bits_truncate(old_evts);
                new_evts |= evts;
                (game_id, channel_id, guild_id, new_evts.bits)
            }
            Err(_) => {
                let guild_id = guild_id.map(|g| g.0 as i64);
                (pk.0, pk.1, guild_id, evts.bits)
            }
        };

        diesel::replace_into(subscriptions)
            .values((
                game.eq(game_id),
                channel.eq(channel_id),
                guild.eq(guild_id),
                events.eq(evts),
            ))
            .execute(&conn)?;

        Ok(())
    }

    pub fn remove(&self, game_id: GameId, channel_id: ChannelId, evts: Events) -> Result<()> {
        use crate::schema::subscriptions::dsl::*;

        type Record = (i32, i64, Option<i64>, i32);

        let conn = self.pool.get()?;

        let pk = (game_id as i32, channel_id.0 as i64);
        let first = subscriptions.find(pk).first::<Record>(&conn);

        if let Ok((game_id, channel_id, guild_id, old_evts)) = first {
            let mut new_evts = Events::from_bits_truncate(old_evts);
            new_evts.remove(evts);

            if new_evts.is_empty() {
                let pred = game.eq(game_id).and(channel.eq(channel_id));
                let filter = subscriptions.filter(pred);
                diesel::delete(filter).execute(&conn)?;

                {
                    use crate::schema::subscriptions_exclude_mods::dsl::*;
                    let pred = game.eq(game_id).and(channel.eq(channel_id));
                    let filter = subscriptions_exclude_mods.filter(pred);
                    diesel::delete(filter).execute(&conn)?;
                }
            } else {
                diesel::replace_into(subscriptions)
                    .values((
                        game.eq(game_id),
                        channel.eq(channel_id),
                        guild.eq(guild_id),
                        events.eq(new_evts.bits),
                    ))
                    .execute(&conn)?;
            }
        }

        Ok(())
    }

    pub fn mute_mod(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
        id: u32,
    ) -> Result<()> {
        use crate::schema::subscriptions_exclude_mods::dsl::*;

        let conn = self.pool.get()?;

        diesel::insert_into(subscriptions_exclude_mods)
            .values((
                game.eq(game_id as i32),
                channel.eq(channel_id.0 as i64),
                guild.eq(guild_id.map(|g| g.0 as i64)),
                mod_id.eq(id as i32),
            ))
            .execute(&conn)?;
        Ok(())
    }

    pub fn unmute_mod(&self, game_id: GameId, channel_id: ChannelId, id: u32) -> Result<()> {
        use crate::schema::subscriptions_exclude_mods::dsl::*;

        let conn = self.pool.get()?;

        let filter = subscriptions_exclude_mods.filter(
            game.eq(game_id as i32)
                .and(channel.eq(channel_id.0 as i64))
                .and(mod_id.eq(id as i32)),
        );
        diesel::delete(filter).execute(&conn)?;
        Ok(())
    }
}

pub fn init_db(database_url: String) -> Result<DbPool> {
    let mgr = ConnectionManager::new(database_url);
    let pool = Pool::new(mgr)?;

    embedded_migrations::run_with_output(&pool.get()?, &mut std::io::stdout())?;

    Ok(pool)
}

pub fn load_blocked(pool: &DbPool) -> Result<Blocked> {
    use crate::schema::blocked_guilds::dsl::*;
    use crate::schema::blocked_users::dsl::*;

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
    use crate::schema::settings::dsl::*;

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
