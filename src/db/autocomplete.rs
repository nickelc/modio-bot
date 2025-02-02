use diesel::prelude::*;
use diesel::sql_types::Text;
use tokio::task::block_in_place;

use super::types::{ApiAccessOptions, GameId};
use super::{schema, DbPool, Result};

#[derive(Debug, Insertable)]
#[diesel(table_name = schema::games)]
pub struct Game<'a> {
    pub id: GameId,
    pub name: &'a str,
    pub name_id: &'a str,
    pub api_access_options: ApiAccessOptions,
}

pub fn replace_games(pool: &DbPool, records: &[Game<'_>]) -> Result<()> {
    let conn = &mut pool.get()?;

    block_in_place(|| {
        use diesel::result::Error;
        use schema::games::dsl::*;

        conn.transaction::<_, Error, _>(|conn| {
            let ids = records.iter().map(|r| r.id).collect::<Vec<_>>();
            let filter = games.filter(id.ne_all(ids));
            let num = diesel::delete(filter).execute(conn)?;
            if num > 0 {
                tracing::info!("Deleted {num} games.");
            }

            let num = diesel::replace_into(games).values(records).execute(conn)?;
            tracing::info!("Replaced {num} games.");
            Ok(())
        })?;

        Ok(())
    })
}

pub fn games_by_name(pool: &DbPool, value: &str) -> Result<Vec<(GameId, String)>> {
    use schema::games::dsl::*;

    let conn = &mut pool.get()?;

    let result = games
        .select((id, name))
        .filter(name.like(format!("{value}%")).and(autocomplete.eq(true)))
        .limit(25)
        .load(conn)?;

    Ok(result)
}

pub fn games_by_name_id(pool: &DbPool, value: &str) -> Result<Vec<(GameId, String)>> {
    use schema::games::dsl::*;

    let conn = &mut pool.get()?;

    let result = games
        .select((id, "@".into_sql::<Text>().concat(name_id)))
        .filter(name_id.like(format!("{value}%")).and(autocomplete.eq(true)))
        .limit(25)
        .load(conn)?;

    Ok(result)
}
