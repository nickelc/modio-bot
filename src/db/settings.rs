use diesel::prelude::*;
use tokio::task::block_in_place;

use super::{schema, DbPool, GameId, GuildId, Result};

#[derive(Clone)]
pub struct Settings {
    pub pool: DbPool,
}

impl Settings {
    pub fn set_game(&self, guild_id: GuildId, game_id: GameId) -> Result<()> {
        use diesel::result::Error;
        use schema::settings::dsl::*;

        let change = (guild.eq(guild_id as i64), game.eq(game_id as i32));

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            conn.transaction::<_, Error, _>(|conn| {
                diesel::replace_into(settings)
                    .values(&change)
                    .execute(conn)?;
                Ok(())
            })?;
            Ok(())
        })
    }

    pub fn game(&self, guild_id: GuildId) -> Result<Option<GameId>> {
        use schema::settings::dsl::*;

        let guild_id = guild_id as i64;

        let conn = &mut self.pool.get()?;
        let value = settings
            .select(game)
            .filter(guild.eq(guild_id))
            .first::<Option<i32>>(conn)
            .optional()?
            .flatten()
            .map(|id| id as u32);

        Ok(value)
    }

    pub fn cleanup(&self, guilds: &[GuildId]) -> Result<()> {
        use schema::settings::dsl::*;

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            let it = guilds.iter().map(|g| *g as i64);
            let ids = it.collect::<Vec<_>>();
            let filter = settings.filter(guild.ne_all(ids));
            match diesel::delete(filter).execute(conn) {
                Ok(num) => tracing::info!("Deleted {num} guild(s)."),
                Err(e) => tracing::error!("{e}"),
            }

            Ok(())
        })
    }
}
