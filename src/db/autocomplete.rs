use diesel::prelude::*;
use diesel::sql_types::Text;

use super::types::GameId;
use super::{schema, DbPool, Result};

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
