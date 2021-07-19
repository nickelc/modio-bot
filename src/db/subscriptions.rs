use std::collections::HashMap;
use std::collections::HashSet;

use diesel::prelude::*;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use tokio::task::block_in_place;

use super::{schema, DbPool, GameId, Result};

pub type ExcludedMods = HashSet<u32>;
pub type ExcludedUsers = HashSet<String>;
pub type Tags = HashSet<String>;
pub type Subscription = (
    ChannelId,
    Tags,
    Option<GuildId>,
    Events,
    ExcludedMods,
    ExcludedUsers,
);

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

impl Default for Events {
    fn default() -> Self {
        Events::ALL
    }
}

impl Subscriptions {
    pub fn cleanup(&self, guilds: &[GuildId]) -> Result<()> {
        use schema::subscriptions::dsl::*;

        block_in_place(|| {
            let conn = self.pool.get()?;
            let it = guilds.iter().map(|g| g.0 as i64);
            let ids = it.collect::<Vec<_>>();
            let filter = subscriptions.filter(guild.ne_all(&ids));
            let num = diesel::delete(filter).execute(&conn)?;
            tracing::info!("Deleted {} subscription(s).", num);

            {
                use schema::subscriptions_exclude_mods::dsl::*;
                let filter = subscriptions_exclude_mods.filter(guild.ne_all(&ids));
                let num = diesel::delete(filter).execute(&conn)?;
                tracing::info!("Deleted {} excluded mods.", num);
            }
            {
                use schema::subscriptions_exclude_users::dsl::*;
                let filter = subscriptions_exclude_users.filter(guild.ne_all(&ids));
                let num = diesel::delete(filter).execute(&conn)?;
                tracing::info!("Deleted {} excluded users.", num);
            }
            Ok(())
        })
    }

    pub fn load(&self) -> Result<HashMap<GameId, Vec<Subscription>>> {
        use super::Error;
        use schema::subscriptions::dsl::*;

        type Record = (i32, i64, String, Option<i64>, i32);

        let (list, mut excluded_mods, mut excluded_users) = block_in_place::<_, Result<_>>(|| {
            let conn = self.pool.get()?;

            conn.transaction::<_, Error, _>(|| {
                let list = subscriptions.load::<Record>(&conn)?;

                let excluded_mods = self.load_excluded_mods()?;
                let excluded_users = self.load_excluded_users()?;

                Ok((list, excluded_mods, excluded_users))
            })
        })?;

        Ok(list.into_iter().fold(
            HashMap::new(),
            |mut map, (game_id, channel_id, _tags, guild_id, evt)| {
                let game_id = game_id as GameId;
                let channel_id = ChannelId(channel_id as u64);
                let _tags = _tags
                    .split('\n')
                    .filter(|t| !t.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
                let guild_id = guild_id.map(|id| GuildId(id as u64));
                let evt = Events::from_bits_truncate(evt);
                let excluded_mods = excluded_mods
                    .remove(&(game_id, channel_id))
                    .unwrap_or_default();
                let excluded_users = excluded_users
                    .remove(&(game_id, channel_id))
                    .unwrap_or_default();
                map.entry(game_id).or_default().push((
                    channel_id,
                    _tags,
                    guild_id,
                    evt,
                    excluded_mods,
                    excluded_users,
                ));
                map
            },
        ))
    }

    fn load_excluded_mods(&self) -> Result<HashMap<(GameId, ChannelId), ExcludedMods>> {
        use schema::subscriptions_exclude_mods::dsl::*;

        type Record = (i32, i64, Option<i64>, i32);

        let list = block_in_place::<_, Result<_>>(|| {
            let conn = self.pool.get()?;
            Ok(subscriptions_exclude_mods.load::<Record>(&conn)?)
        })?;

        Ok(list
            .into_iter()
            .fold(HashMap::new(), |mut map, (game_id, channel_id, _, mid)| {
                let key = (game_id as GameId, ChannelId(channel_id as u64));
                map.entry(key).or_default().insert(mid as u32);
                map
            }))
    }

    fn load_excluded_users(&self) -> Result<HashMap<(GameId, ChannelId), ExcludedUsers>> {
        use schema::subscriptions_exclude_users::dsl::*;

        type Record = (i32, i64, Option<i64>, String);

        let list = block_in_place::<_, Result<_>>(|| {
            let conn = self.pool.get()?;
            Ok(subscriptions_exclude_users.load::<Record>(&conn)?)
        })?;

        Ok(list
            .into_iter()
            .fold(HashMap::new(), |mut map, (game_id, channel_id, _, name)| {
                let key = (game_id as GameId, ChannelId(channel_id as u64));
                map.entry(key).or_default().insert(name);
                map
            }))
    }

    pub fn list_for_channel(&self, channel_id: ChannelId) -> Result<Vec<(GameId, Tags, Events)>> {
        use schema::subscriptions::dsl::*;

        let records = block_in_place::<_, Result<_>>(|| {
            let conn = self.pool.get()?;

            let records = subscriptions
                .select((game, tags, events))
                .filter(channel.eq(channel_id.0 as i64))
                .load::<(i32, String, i32)>(&conn)?;
            Ok(records)
        })?;

        let records = records
            .into_iter()
            .map(|(game_id, _tags, evts)| {
                let _tags = _tags
                    .split('\n')
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
                (game_id as u32, _tags, Events::from_bits_truncate(evts))
            })
            .collect();

        Ok(records)
    }

    pub fn list_excluded_mods(
        &self,
        channel_id: ChannelId,
    ) -> Result<HashMap<GameId, ExcludedMods>> {
        use schema::subscriptions_exclude_mods::dsl::*;

        let records = block_in_place::<_, Result<_>>(|| {
            let conn = self.pool.get()?;

            let records = subscriptions_exclude_mods
                .select((game, mod_id))
                .filter(channel.eq(channel_id.0 as i64))
                .load::<(i32, i32)>(&conn)?;
            Ok(records)
        })?;

        let records: HashMap<GameId, ExcludedMods> =
            records
                .into_iter()
                .fold(HashMap::new(), |mut map, (game_id, mid)| {
                    map.entry(game_id as GameId).or_default().insert(mid as u32);
                    map
                });
        Ok(records)
    }

    pub fn list_excluded_users(
        &self,
        channel_id: ChannelId,
    ) -> Result<HashMap<GameId, ExcludedUsers>> {
        use schema::subscriptions_exclude_users::dsl::*;

        let records = block_in_place::<_, Result<_>>(|| {
            let conn = self.pool.get()?;

            let records = subscriptions_exclude_users
                .select((game, user))
                .filter(channel.eq(channel_id.0 as i64))
                .load::<(i32, String)>(&conn)?;
            Ok(records)
        })?;

        let records: HashMap<GameId, ExcludedUsers> =
            records
                .into_iter()
                .fold(HashMap::new(), |mut map, (game_id, name)| {
                    map.entry(game_id as GameId).or_default().insert(name);
                    map
                });
        Ok(records)
    }

    pub fn add(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        _tags: Tags,
        guild_id: Option<GuildId>,
        evts: Events,
    ) -> Result<()> {
        use diesel::result::Error;
        use schema::subscriptions::dsl::*;

        type Record = (i32, i64, String, Option<i64>, i32);

        let game_id = game_id as i32;
        let channel_id = channel_id.0 as i64;

        let mut _tags = _tags.into_iter().collect::<Vec<_>>();
        _tags.sort();
        let _tags = _tags.join("\n");

        let pk = (game_id, channel_id, _tags.clone());

        block_in_place(|| {
            let conn = self.pool.get()?;

            conn.transaction::<_, Error, _>(|| {
                let first = subscriptions.find(pk).first::<Record>(&conn);

                let (game_id, channel_id, _tags, guild_id, evts) = match first {
                    Ok((game_id, channel_id, _tags, guild_id, old_evts)) => {
                        let mut new_evts = Events::from_bits_truncate(old_evts);
                        new_evts |= evts;
                        (game_id, channel_id, _tags, guild_id, new_evts.bits)
                    }
                    Err(_) => {
                        let guild_id = guild_id.map(|g| g.0 as i64);
                        (game_id, channel_id, _tags, guild_id, evts.bits)
                    }
                };

                let values = (
                    game.eq(game_id),
                    channel.eq(channel_id),
                    tags.eq(_tags),
                    guild.eq(guild_id),
                    events.eq(evts),
                );
                diesel::replace_into(subscriptions)
                    .values(values)
                    .execute(&conn)
            })?;

            Ok(())
        })
    }

    pub fn remove(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        _tags: Tags,
        evts: Events,
    ) -> Result<()> {
        use diesel::result::Error;
        use schema::subscriptions::dsl::*;

        type Record = (i32, i64, String, Option<i64>, i32);

        let mut _tags = _tags.into_iter().collect::<Vec<_>>();
        _tags.sort();
        let _tags = _tags.join("\n");

        let pk = (game_id as i32, channel_id.0 as i64, _tags);

        block_in_place(|| {
            let conn = self.pool.get()?;

            conn.transaction::<_, Error, _>(|| {
                let first = subscriptions.find(pk).first::<Record>(&conn);

                if let Ok((game_id, channel_id, _tags, guild_id, old_evts)) = first {
                    let mut new_evts = Events::from_bits_truncate(old_evts);
                    new_evts.remove(evts);

                    if new_evts.is_empty() {
                        let pred = game
                            .eq(game_id)
                            .and(channel.eq(channel_id))
                            .and(tags.eq(_tags));
                        let filter = subscriptions.filter(pred);
                        diesel::delete(filter).execute(&conn)?;

                        let count = subscriptions
                            .select(diesel::dsl::count_star())
                            .filter(game.eq(game_id).and(channel.eq(channel_id)))
                            .first::<i64>(&conn)?;

                        if count == 0 {
                            {
                                use schema::subscriptions_exclude_mods::dsl::*;
                                let pred = game.eq(game_id).and(channel.eq(channel_id));
                                let filter = subscriptions_exclude_mods.filter(pred);
                                diesel::delete(filter).execute(&conn)?;
                            }
                            {
                                use schema::subscriptions_exclude_users::dsl::*;
                                let pred = game.eq(game_id).and(channel.eq(channel_id));
                                let filter = subscriptions_exclude_users.filter(pred);
                                diesel::delete(filter).execute(&conn)?;
                            }
                        }
                    } else {
                        let values = (
                            game.eq(game_id),
                            channel.eq(channel_id),
                            tags.eq(_tags),
                            guild.eq(guild_id),
                            events.eq(new_evts.bits),
                        );
                        diesel::replace_into(subscriptions)
                            .values(values)
                            .execute(&conn)?;
                    }
                }
                Ok(())
            })?;
            Ok(())
        })
    }

    pub fn mute_mod(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
        id: u32,
    ) -> Result<()> {
        use schema::subscriptions_exclude_mods::dsl::*;

        block_in_place(|| {
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
        })
    }

    pub fn unmute_mod(&self, game_id: GameId, channel_id: ChannelId, id: u32) -> Result<()> {
        use schema::subscriptions_exclude_mods::dsl::*;

        block_in_place(|| {
            let conn = self.pool.get()?;

            let filter = subscriptions_exclude_mods.filter(
                game.eq(game_id as i32)
                    .and(channel.eq(channel_id.0 as i64))
                    .and(mod_id.eq(id as i32)),
            );
            diesel::delete(filter).execute(&conn)?;
            Ok(())
        })
    }

    pub fn mute_user(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
        name: &str,
    ) -> Result<()> {
        use schema::subscriptions_exclude_users::dsl::*;

        block_in_place(|| {
            let conn = self.pool.get()?;

            diesel::insert_into(subscriptions_exclude_users)
                .values((
                    game.eq(game_id as i32),
                    channel.eq(channel_id.0 as i64),
                    guild.eq(guild_id.map(|g| g.0 as i64)),
                    user.eq(name),
                ))
                .execute(&conn)?;
            Ok(())
        })
    }

    pub fn unmute_user(&self, game_id: GameId, channel_id: ChannelId, name: &str) -> Result<()> {
        use schema::subscriptions_exclude_users::dsl::*;

        block_in_place(|| {
            let conn = self.pool.get()?;

            let filter = subscriptions_exclude_users.filter(
                game.eq(game_id as i32)
                    .and(channel.eq(channel_id.0 as i64))
                    .and(user.eq(name)),
            );
            diesel::delete(filter).execute(&conn)?;
            Ok(())
        })
    }
}
