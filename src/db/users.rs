use diesel::prelude::*;
use modio::auth::Token;
use serenity::model::id::UserId;

use super::{schema, DbPool, Result};

type Record = (i64, String, String, String);

#[derive(Clone)]
pub struct Users {
    pub pool: DbPool,
}

impl Users {
    pub fn find_token(&self, user_id: UserId) -> Result<Option<Token>> {
        use schema::users::dsl::*;

        let conn = self.pool.get()?;
        let token = users
            .find(user_id.0 as i64)
            .first::<Record>(&conn)
            .optional()?
            .map(|(_, token, _, _)| Token {
                value: token,
                expired_at: None,
            });

        Ok(token)
    }
}
