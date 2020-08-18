use std::iter::IntoIterator;

use diesel::prelude::*;
use serenity::model::id::MessageId;

use super::{schema, DbPool, GameId, ModId, Result};

type Record = (i32, i32);

#[derive(Clone)]
pub struct Messages {
    pub pool: DbPool,
}

impl Messages {
    pub fn find(&self, message_id: MessageId) -> Result<Option<(GameId, ModId)>> {
        use schema::messages::dsl::*;

        let conn = self.pool.get()?;
        let msg = messages
            .select((game_id, mod_id))
            .filter(id.eq(message_id.0 as i64))
            .first::<Record>(&conn)
            .optional()?
            .map(|(game, mod_)| (game as u32, mod_ as u32));

        Ok(msg)
    }

    pub fn new_messages<T>(&self, msgs: T) -> Result<()>
    where
        T: IntoIterator<Item = (MessageId, GameId, ModId)>,
    {
        use diesel::result::Error;
        use schema::messages::dsl::*;

        let conn = self.pool.get()?;

        let values = msgs.into_iter().map(|(mid, g, m)| {
            (
                id.eq(mid.0 as i64),
                game_id.eq(g as i32),
                mod_id.eq(m as i32),
            )
        });
        conn.transaction(|| {
            for values in values {
                diesel::insert_into(messages)
                    .values(values)
                    .execute(&conn)?;
            }
            Ok::<_, Error>(())
        })?;

        Ok(())
    }
}
