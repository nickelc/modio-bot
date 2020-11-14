use std::collections::HashMap;

use diesel::prelude::*;
use serenity::model::id::GuildId;
use tokio::task::block_in_place;

use super::{schema, DbPool, GameId, Result};
use schema::settings;

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
    fn persist(&self, change: ChangeSettings) -> Result<()> {
        use diesel::result::Error;
        use schema::settings::dsl::*;

        block_in_place(|| {
            let conn = self.pool.get()?;
            let target = settings.filter(guild.eq(change.guild));

            conn.transaction::<_, Error, _>(|| {
                let query = diesel::update(target).set(&change);

                if query.execute(&conn)? == 0 {
                    let query = diesel::insert_into(settings).values(&change);
                    query.execute(&conn)?;
                }
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

    pub fn prefix(&self, guild: Option<GuildId>) -> Option<String> {
        self.data.get(&guild?).and_then(|s| s.prefix.clone())
    }

    pub fn set_prefix(&mut self, guild: GuildId, prefix: Option<String>) -> Result<()> {
        let change = (guild, prefix.clone());
        self.persist(change.into())?;

        self.data.entry(guild).or_default().prefix = prefix;
        Ok(())
    }
}

pub fn load_settings(pool: &DbPool, guilds: &[GuildId]) -> Result<HashMap<GuildId, GuildSettings>> {
    use schema::settings::dsl::*;

    type Record = (i64, Option<i32>, Option<String>);

    let list = block_in_place::<_, Result<_>>(|| {
        let conn = pool.get()?;

        let it = guilds.iter().map(|g| g.0 as i64);
        let ids = it.collect::<Vec<_>>();
        let filter = settings.filter(guild.ne_all(ids));
        match diesel::delete(filter).execute(&conn) {
            Ok(num) => tracing::info!("Deleted {} guild(s).", num),
            Err(e) => tracing::error!("{}", e),
        }

        Ok(settings.load::<Record>(&conn).unwrap_or_default())
    })?;

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
