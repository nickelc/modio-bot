use std::collections::HashMap;

use diesel::prelude::*;
use tokio::task::block_in_place;

use super::{schema, DbPool, GameId, GuildId, Result};
use schema::settings;

#[derive(Debug, Default)]
pub struct GuildSettings {
    game: Option<GameId>,
}

#[derive(Insertable)]
#[diesel(table_name = settings)]
#[allow(clippy::option_option)]
struct ChangeSettings {
    pub guild: i64,
    pub game: Option<Option<i32>>,
}

pub struct Settings {
    pub pool: DbPool,
    pub data: HashMap<GuildId, GuildSettings>,
}

impl Settings {
    fn persist(&self, change: ChangeSettings) -> Result<()> {
        use diesel::result::Error;
        use schema::settings::dsl::*;

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

    pub fn game(&self, guild: GuildId) -> Option<GameId> {
        self.data.get(&guild).and_then(|s| s.game)
    }

    pub fn set_game(&mut self, guild: GuildId, game: GameId) -> Result<()> {
        let change = (guild, game);
        self.persist(change.into())?;

        self.data.entry(guild).or_default().game = Some(game);
        Ok(())
    }
}

pub fn load_settings(pool: &DbPool, guilds: &[GuildId]) -> Result<HashMap<GuildId, GuildSettings>> {
    use schema::settings::dsl::*;

    type Record = (i64, Option<i32>);

    let list = block_in_place::<_, Result<_>>(|| {
        let conn = &mut pool.get()?;

        let it = guilds.iter().map(|g| *g as i64);
        let ids = it.collect::<Vec<_>>();
        let filter = settings.filter(guild.ne_all(ids));
        match diesel::delete(filter).execute(conn) {
            Ok(num) => tracing::info!("Deleted {num} guild(s)."),
            Err(e) => tracing::error!("{e}"),
        }

        Ok(settings.load::<Record>(conn).unwrap_or_default())
    })?;

    let mut map = HashMap::new();
    for r in list {
        map.insert(
            r.0 as u64,
            GuildSettings {
                game: r.1.map(|id| id as u32),
            },
        );
    }
    Ok(map)
}

impl From<(GuildId, GameId)> for ChangeSettings {
    fn from((guild, game): (GuildId, GameId)) -> Self {
        Self {
            guild: guild as i64,
            game: Some(Some(game as i32)),
        }
    }
}
