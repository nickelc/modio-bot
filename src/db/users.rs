use std::time::Duration;

use diesel::prelude::*;
use diesel::sql_types::{Text, Timestamp};
use modio::auth::Token;
use serenity::model::id::UserId;

use super::{schema, DbPool, Result};

type Record = (i64, String, String, String);

sql_function!(fn datetime(timestring: Text, modifier: Text) -> Timestamp);

pub struct NewToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: Duration,
}

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

    pub fn save_token(&self, user_id: UserId, token: NewToken) -> Result<()> {
        use schema::users::dsl::*;

        let conn = self.pool.get()?;

        let user_id = user_id.0 as i64;
        let target = users.filter(id.eq(user_id));

        let values = (
            access_token.eq(token.access_token),
            refresh_token.eq(token.refresh_token),
        );
        let values = (
            values,
            expired_at.eq(datetime(
                "now",
                format!("+{} seconds", token.expires_in.as_secs()),
            )),
        );

        let query = diesel::update(target).set(values.clone());
        if query.execute(&conn)? == 0 {
            let values = (id.eq(user_id), values);
            diesel::insert_into(users).values(values).execute(&conn)?;
        }
        Ok(())
    }

    pub fn delete(&self, user_id: UserId) -> Result<()> {
        use schema::users::dsl::*;

        let conn = self.pool.get()?;
        let target = users.filter(id.eq(user_id.0 as i64));
        diesel::delete(target).execute(&conn)?;

        Ok(())
    }

    pub fn tokens_to_refresh(&self) -> Result<Vec<(UserId, String)>> {
        use schema::users::dsl::*;

        let conn = self.pool.get()?;
        let result = users
            .select((id, refresh_token))
            .filter(expired_at.lt(datetime("now", "+4 hours")))
            .load(&conn)?
            .into_iter()
            .map(|(uid, token): (i64, String)| (UserId::from(uid as u64), token))
            .collect();

        Ok(result)
    }
}
