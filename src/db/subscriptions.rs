use std::collections::{BTreeMap, HashMap, HashSet};

use diesel::prelude::*;
use tokio::task::block_in_place;

mod events;
mod tags;

use super::types::{ChannelId, GameId, GuildId, ModId};
use super::{schema, DbPool, Result};

pub type ExcludedMods = HashSet<ModId>;
pub type ExcludedUsers = HashSet<String>;
pub type ExcludedModsMap = HashMap<(GameId, ChannelId), ExcludedMods>;
pub type ExcludedUsersMap = HashMap<(GameId, ChannelId), ExcludedUsers>;
pub type Subscription = (ChannelId, Tags, GuildId, Events);
pub type GroupedSubscriptions = BTreeMap<ChannelId, Vec<(GameId, Tags, Events)>>;

pub use events::Events;
pub use tags::Tags;

#[derive(Clone)]
pub struct Subscriptions {
    pub pool: DbPool,
}

impl Subscriptions {
    pub fn cleanup(&self, guilds: &[GuildId]) -> Result<()> {
        use schema::subscriptions::dsl::*;

        block_in_place(|| {
            let conn = &mut self.pool.get()?;
            let filter = subscriptions.filter(guild.ne_all(guilds));
            let num = diesel::delete(filter).execute(conn)?;
            tracing::info!("Deleted {num} subscription(s).");

            {
                use schema::subscriptions_exclude_mods::dsl::*;
                let filter = subscriptions_exclude_mods.filter(guild.ne_all(guilds));
                let num = diesel::delete(filter).execute(conn)?;
                tracing::info!("Deleted {num} excluded mods.");
            }
            {
                use schema::subscriptions_exclude_users::dsl::*;
                let filter = subscriptions_exclude_users.filter(guild.ne_all(guilds));
                let num = diesel::delete(filter).execute(conn)?;
                tracing::info!("Deleted {num} excluded users.");
            }
            Ok(())
        })
    }

    pub fn cleanup_unknown_channels(&self, channels: &[ChannelId]) -> Result<()> {
        use schema::subscriptions::dsl::*;

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            let filter = subscriptions.filter(channel.eq_any(channels));
            let num = diesel::delete(filter).execute(conn)?;
            tracing::info!("Deleted {num} subscription(s).");

            {
                use schema::subscriptions_exclude_mods::dsl::*;
                let filter = subscriptions_exclude_mods.filter(channel.eq_any(channels));
                let num = diesel::delete(filter).execute(conn)?;
                if num > 0 {
                    tracing::info!("Deleted {num} excluded mod entries.");
                }
            }
            {
                use schema::subscriptions_exclude_users::dsl::*;
                let filter = subscriptions_exclude_users.filter(channel.eq_any(channels));
                let num = diesel::delete(filter).execute(conn)?;
                if num > 0 {
                    tracing::info!("Deleted {num} excluded user entries.");
                }
            }
            Ok(())
        })
    }

    pub fn get_channels(&self) -> Result<HashSet<ChannelId>> {
        use schema::subscriptions::dsl::*;

        let channels = block_in_place::<_, Result<_>>(|| {
            let conn = &mut self.pool.get()?;

            Ok(subscriptions.select(channel).distinct().load(conn)?)
        })?;

        Ok(channels.into_iter().collect())
    }

    pub fn load(
        &self,
    ) -> Result<(
        HashMap<GameId, Vec<Subscription>>,
        ExcludedModsMap,
        ExcludedUsersMap,
    )> {
        use super::Error;
        use schema::subscriptions::dsl::*;

        type Record = (GameId, ChannelId, Tags, GuildId, Events);

        let (list, excluded_mods, excluded_users) = block_in_place::<_, Result<_>>(|| {
            let conn = &mut self.pool.get()?;

            conn.transaction::<_, Error, _>(|conn| {
                let list = subscriptions.load::<Record>(conn)?;

                let excluded_mods = self.load_excluded_mods()?;
                let excluded_users = self.load_excluded_users()?;

                Ok((list, excluded_mods, excluded_users))
            })
        })?;

        let subs = list.into_iter().fold(
            HashMap::<_, Vec<_>>::new(),
            |mut map, (game_id, channel_id, sub_tags, guild_id, evt)| {
                map.entry(game_id)
                    .or_default()
                    .push((channel_id, sub_tags, guild_id, evt));
                map
            },
        );

        Ok((subs, excluded_mods, excluded_users))
    }

    fn load_excluded_mods(&self) -> Result<ExcludedModsMap> {
        use schema::subscriptions_exclude_mods::dsl::*;

        type Record = (GameId, ChannelId, GuildId, ModId);

        let list = block_in_place::<_, Result<_>>(|| {
            let conn = &mut self.pool.get()?;
            Ok(subscriptions_exclude_mods.load::<Record>(conn)?)
        })?;

        Ok(list
            .into_iter()
            .fold(HashMap::new(), |mut map, (game_id, channel_id, _, mid)| {
                let key = (game_id, channel_id);
                map.entry(key).or_default().insert(mid);
                map
            }))
    }

    fn load_excluded_users(&self) -> Result<ExcludedUsersMap> {
        use schema::subscriptions_exclude_users::dsl::*;

        type Record = (GameId, ChannelId, GuildId, String);

        let list = block_in_place::<_, Result<_>>(|| {
            let conn = &mut self.pool.get()?;
            Ok(subscriptions_exclude_users.load::<Record>(conn)?)
        })?;

        Ok(list
            .into_iter()
            .fold(HashMap::new(), |mut map, (game_id, channel_id, _, name)| {
                let key = (game_id, channel_id);
                map.entry(key).or_default().insert(name);
                map
            }))
    }

    pub fn list_for_overview(
        &self,
        guild_id: GuildId,
    ) -> Result<(GroupedSubscriptions, ExcludedModsMap, ExcludedUsersMap)> {
        let (subs, excluded_mods, excluded_users) = block_in_place::<_, Result<_>>(|| {
            use schema::subscriptions::dsl::*;

            let conn = &mut self.pool.get()?;

            let subs = subscriptions
                .select((channel, game, tags, events))
                .filter(guild.eq(guild_id))
                .load::<(ChannelId, GameId, Tags, Events)>(conn)?;

            let excluded_mods = {
                use schema::subscriptions_exclude_mods::dsl::*;

                subscriptions_exclude_mods
                    .select((channel, game, mod_id))
                    .filter(guild.eq(guild_id))
                    .load::<(ChannelId, GameId, ModId)>(conn)?
            };

            let excluded_users = {
                use schema::subscriptions_exclude_users::dsl::*;

                subscriptions_exclude_users
                    .select((channel, game, user))
                    .filter(guild.eq(guild_id))
                    .load::<(ChannelId, GameId, String)>(conn)?
            };
            Ok((subs, excluded_mods, excluded_users))
        })?;

        let subs = subs.into_iter().fold(
            GroupedSubscriptions::new(),
            |mut map, (channel_id, game_id, tags, events)| {
                map.entry(channel_id)
                    .or_default()
                    .push((game_id, tags, events));
                map
            },
        );

        let excluded_mods = excluded_mods.into_iter().fold(
            ExcludedModsMap::new(),
            |mut map, (channel_id, game_id, mod_id)| {
                map.entry((game_id, channel_id)).or_default().insert(mod_id);
                map
            },
        );

        let excluded_users = excluded_users.into_iter().fold(
            ExcludedUsersMap::new(),
            |mut map, (channel_id, game_id, user)| {
                map.entry((game_id, channel_id)).or_default().insert(user);
                map
            },
        );

        Ok((subs, excluded_mods, excluded_users))
    }

    pub fn list_for_channel(&self, channel_id: ChannelId) -> Result<Vec<(GameId, Tags, Events)>> {
        use schema::subscriptions::dsl::*;

        let records = block_in_place::<_, Result<_>>(|| {
            let conn = &mut self.pool.get()?;

            let records = subscriptions
                .select((game, tags, events))
                .filter(channel.eq(channel_id))
                .load::<(GameId, Tags, Events)>(conn)?;
            Ok(records)
        })?;

        Ok(records)
    }

    pub fn list_excluded_mods(
        &self,
        channel_id: ChannelId,
    ) -> Result<HashMap<GameId, ExcludedMods>> {
        use schema::subscriptions_exclude_mods::dsl::*;

        let records = block_in_place::<_, Result<_>>(|| {
            let conn = &mut self.pool.get()?;

            let records = subscriptions_exclude_mods
                .select((game, mod_id))
                .filter(channel.eq(channel_id))
                .load::<(GameId, ModId)>(conn)?;
            Ok(records)
        })?;

        let records: HashMap<GameId, ExcludedMods> =
            records
                .into_iter()
                .fold(HashMap::new(), |mut map, (game_id, mid)| {
                    map.entry(game_id).or_default().insert(mid);
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
            let conn = &mut self.pool.get()?;

            let records = subscriptions_exclude_users
                .select((game, user))
                .filter(channel.eq(channel_id))
                .load::<(GameId, String)>(conn)?;
            Ok(records)
        })?;

        let records: HashMap<GameId, ExcludedUsers> =
            records
                .into_iter()
                .fold(HashMap::new(), |mut map, (game_id, name)| {
                    map.entry(game_id).or_default().insert(name);
                    map
                });
        Ok(records)
    }

    pub fn add(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        sub_tags: Tags,
        guild_id: GuildId,
        evts: Events,
    ) -> Result<()> {
        use diesel::result::Error;
        use schema::subscriptions::dsl::*;

        type Record = (GameId, ChannelId, Tags, GuildId, Events);

        let pk = (game_id, channel_id, sub_tags.to_string());

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            conn.transaction::<_, Error, _>(|conn| {
                let first = subscriptions.find(pk).first::<Record>(conn);

                let (game_id, channel_id, sub_tags, guild_id, evts) =
                    if let Ok((game_id, channel_id, sub_tags, guild_id, old_evts)) = first {
                        let mut new_evts = old_evts;
                        new_evts |= evts;
                        (game_id, channel_id, sub_tags, guild_id, new_evts)
                    } else {
                        (game_id, channel_id, sub_tags, guild_id, evts)
                    };

                let values = (
                    game.eq(game_id),
                    channel.eq(channel_id),
                    tags.eq(sub_tags),
                    guild.eq(guild_id),
                    events.eq(evts),
                );
                diesel::replace_into(subscriptions)
                    .values(values)
                    .execute(conn)
            })?;

            Ok(())
        })
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn remove(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        sub_tags: Tags,
        evts: Events,
    ) -> Result<()> {
        use diesel::result::Error;
        use schema::subscriptions::dsl::*;

        type Record = (GameId, ChannelId, Tags, GuildId, Events);

        let pk = (game_id, channel_id, sub_tags.to_string());

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            conn.transaction::<_, Error, _>(|conn| {
                let first = subscriptions.find(pk).first::<Record>(conn);

                if let Ok((game_id, channel_id, sub_tags, guild_id, old_evts)) = first {
                    let mut new_evts = old_evts;
                    new_evts.remove(evts);

                    if new_evts.is_empty() {
                        let pred = game
                            .eq(game_id)
                            .and(channel.eq(channel_id))
                            .and(tags.eq(sub_tags));
                        let filter = subscriptions.filter(pred);
                        diesel::delete(filter).execute(conn)?;

                        let count = subscriptions
                            .select(diesel::dsl::count_star())
                            .filter(game.eq(game_id).and(channel.eq(channel_id)))
                            .first::<i64>(conn)?;

                        if count == 0 {
                            {
                                use schema::subscriptions_exclude_mods::dsl::*;
                                let pred = game.eq(game_id).and(channel.eq(channel_id));
                                let filter = subscriptions_exclude_mods.filter(pred);
                                diesel::delete(filter).execute(conn)?;
                            }
                            {
                                use schema::subscriptions_exclude_users::dsl::*;
                                let pred = game.eq(game_id).and(channel.eq(channel_id));
                                let filter = subscriptions_exclude_users.filter(pred);
                                diesel::delete(filter).execute(conn)?;
                            }
                        }
                    } else {
                        let values = (
                            game.eq(game_id),
                            channel.eq(channel_id),
                            tags.eq(sub_tags),
                            guild.eq(guild_id),
                            events.eq(new_evts),
                        );
                        diesel::replace_into(subscriptions)
                            .values(values)
                            .execute(conn)?;
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
        guild_id: GuildId,
        id: ModId,
    ) -> Result<()> {
        use schema::subscriptions_exclude_mods::dsl::*;

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            diesel::insert_into(subscriptions_exclude_mods)
                .values((
                    game.eq(game_id),
                    channel.eq(channel_id),
                    guild.eq(guild_id),
                    mod_id.eq(id),
                ))
                .execute(conn)?;
            Ok(())
        })
    }

    pub fn unmute_mod(&self, game_id: GameId, channel_id: ChannelId, id: ModId) -> Result<()> {
        use schema::subscriptions_exclude_mods::dsl::*;

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            let filter = subscriptions_exclude_mods.filter(
                game.eq(game_id)
                    .and(channel.eq(channel_id))
                    .and(mod_id.eq(id)),
            );
            diesel::delete(filter).execute(conn)?;
            Ok(())
        })
    }

    pub fn mute_user(
        &self,
        game_id: GameId,
        channel_id: ChannelId,
        guild_id: GuildId,
        name: &str,
    ) -> Result<()> {
        use schema::subscriptions_exclude_users::dsl::*;

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            diesel::insert_into(subscriptions_exclude_users)
                .values((
                    game.eq(game_id),
                    channel.eq(channel_id),
                    guild.eq(guild_id),
                    user.eq(name),
                ))
                .execute(conn)?;
            Ok(())
        })
    }

    pub fn unmute_user(&self, game_id: GameId, channel_id: ChannelId, name: &str) -> Result<()> {
        use schema::subscriptions_exclude_users::dsl::*;

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            let filter = subscriptions_exclude_users.filter(
                game.eq(game_id)
                    .and(channel.eq(channel_id))
                    .and(user.eq(name)),
            );
            diesel::delete(filter).execute(conn)?;
            Ok(())
        })
    }
}
