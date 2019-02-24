use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use serenity::client::Context;
use serenity::model::channel::Message;
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

pub fn init_db(database_url: String) -> Result<DbPool> {
    let mgr = ConnectionManager::new(database_url);
    let pool = Pool::new(mgr)?;

    embedded_migrations::run_with_output(&pool.get()?, &mut std::io::stdout())?;

    Ok(pool)
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
