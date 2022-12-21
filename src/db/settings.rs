use diesel::prelude::*;
use tokio::task::block_in_place;

use super::types::{GameId, GuildId};
use super::{schema, DbPool, Result};

#[derive(Clone)]
pub struct Settings {
    pub pool: DbPool,
}

impl Settings {
    pub fn set_game(&self, guild_id: GuildId, game_id: GameId) -> Result<()> {
        use diesel::result::Error;
        use schema::settings::dsl::*;

        let change = (guild.eq(guild_id), game.eq(game_id));

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

        let conn = &mut self.pool.get()?;
        let value = settings
            .select(game)
            .filter(guild.eq(guild_id))
            .first::<Option<GameId>>(conn)
            .optional()?
            .flatten();

        Ok(value)
    }

    pub fn cleanup(&self, guilds: &[GuildId]) -> Result<()> {
        use schema::settings::dsl::*;

        block_in_place(|| {
            let conn = &mut self.pool.get()?;

            let filter = settings.filter(guild.ne_all(guilds));
            match diesel::delete(filter).execute(conn) {
                Ok(num) => tracing::info!("Deleted {num} guild(s)."),
                Err(e) => tracing::error!("{e}"),
            }

            Ok(())
        })
    }
}
