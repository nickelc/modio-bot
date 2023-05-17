use diesel::backend::RawValue;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, ToSql};
use diesel::sql_types::Integer;
use diesel::sqlite::Sqlite;

#[derive(Clone, Copy, Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = Integer)]
#[repr(transparent)]
pub struct Events(i32);

bitflags::bitflags! {
    impl Events: i32 {
        const NEW = 0b0001;
        const UPD = 0b0010;
        const ALL = Self::NEW.bits() | Self::UPD.bits();
    }
}

impl Default for Events {
    fn default() -> Self {
        Self::ALL
    }
}

impl FromSql<Integer, Sqlite> for Events {
    fn from_sql(bytes: RawValue<'_, Sqlite>) -> deserialize::Result<Self> {
        let bits = i32::from_sql(bytes)?;
        Ok(Self::from_bits_truncate(bits))
    }
}

impl ToSql<Integer, Sqlite> for Events {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.0);
        Ok(serialize::IsNull::No)
    }
}
