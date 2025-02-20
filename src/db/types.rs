use std::fmt;
use std::ops::Deref;

use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, ToSql};
use diesel::sql_types::{BigInt, Integer};
use diesel::sqlite::Sqlite;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, UserMarker};
use twilight_model::id::Id;

#[derive(Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd, AsExpression, FromSqlRow)]
#[diesel(sql_type = BigInt)]
pub struct GameId(pub modio::types::id::GameId);

#[derive(Eq, Hash, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = BigInt)]
pub struct ModId(pub modio::types::id::ModId);

#[derive(Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd, AsExpression, FromSqlRow)]
#[diesel(sql_type = BigInt)]
pub struct ChannelId(pub Id<ChannelMarker>);

#[derive(Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd, AsExpression, FromSqlRow)]
#[diesel(sql_type = BigInt)]
pub struct GuildId(pub Id<GuildMarker>);

#[derive(Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd, AsExpression, FromSqlRow)]
#[diesel(sql_type = BigInt)]
pub struct UserId(pub Id<UserMarker>);

#[derive(Copy, Clone, Eq, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Integer)]
pub struct ApiAccessOptions(pub modio::types::games::ApiAccessOptions);

impl Deref for GameId {
    type Target = modio::types::id::GameId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for ModId {
    type Target = modio::types::id::ModId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for ChannelId {
    type Target = Id<ChannelMarker>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0.get(), f)
    }
}

impl fmt::Display for ModId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for ModId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0.get(), f)
    }
}

impl fmt::Display for ChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for ChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0.get(), f)
    }
}

impl fmt::Display for GuildId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for GuildId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0.get(), f)
    }
}

impl fmt::Debug for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0.get(), f)
    }
}

impl fmt::Debug for ApiAccessOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for ApiAccessOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromSql<BigInt, Sqlite> for GameId {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let id = i64::from_sql(bytes)?;
        Ok(Self(TryFrom::try_from(id)?))
    }
}

impl ToSql<BigInt, Sqlite> for GameId {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(i64::try_from(self.0.get())?);
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<BigInt, Sqlite> for ModId {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let id = i64::from_sql(bytes)?;
        Ok(Self(TryFrom::try_from(id)?))
    }
}

impl ToSql<BigInt, Sqlite> for ModId {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(i64::try_from(self.0.get())?);
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<BigInt, Sqlite> for ChannelId {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let id = i64::from_sql(bytes)?;
        Ok(Self(Id::try_from(id)?))
    }
}

impl ToSql<BigInt, Sqlite> for ChannelId {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(i64::try_from(self.0.get())?);
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<BigInt, Sqlite> for GuildId {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let id = i64::from_sql(bytes)?;
        Ok(Self(Id::try_from(id)?))
    }
}

impl ToSql<BigInt, Sqlite> for GuildId {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(i64::try_from(self.0.get())?);
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<BigInt, Sqlite> for UserId {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let id = i64::from_sql(bytes)?;
        Ok(Self(Id::try_from(id)?))
    }
}

impl ToSql<BigInt, Sqlite> for UserId {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(i64::try_from(self.0.get())?);
        Ok(serialize::IsNull::No)
    }
}

impl ToSql<Integer, Sqlite> for ApiAccessOptions {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(i32::from(self.0.bits()));
        Ok(serialize::IsNull::No)
    }
}
