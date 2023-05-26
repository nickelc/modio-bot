use std::collections::btree_set::IntoIter;
use std::collections::BTreeSet;
use std::fmt;
use std::ops::Deref;

use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;

#[derive(Debug, Default, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct Tags(pub BTreeSet<String>);

impl Tags {
    pub fn from_csv(s: &str) -> Self {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .trim(csv::Trim::All)
            .from_reader(s.as_bytes());
        let mut record = csv::StringRecord::new();
        match rdr.read_record(&mut record) {
            Ok(true) => record
                .iter()
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            _ => Tags::default(),
        }
    }

    /// Create tags from newline separated tags string.
    pub fn from_str(s: &str) -> Self {
        s.split('\n')
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }

    /// Returns a pair of tags, all hidden tags that begin with `*`, and all of the rest.
    pub fn partition(self) -> (Self, Self) {
        self.0.into_iter().partition(|s| s.starts_with('*'))
    }
}

impl fmt::Display for Tags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tags = self
            .0
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n");
        fmt::Display::fmt(&tags, f)
    }
}

impl Deref for Tags {
    type Target = BTreeSet<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromSql<Text, Sqlite> for Tags {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let tags = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        Ok(Tags::from_str(&tags))
    }
}

impl ToSql<Text, Sqlite> for Tags {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(serialize::IsNull::No)
    }
}

impl Extend<String> for Tags {
    fn extend<T: IntoIterator<Item = String>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl FromIterator<String> for Tags {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self(BTreeSet::from_iter(iter))
    }
}

impl IntoIterator for Tags {
    type Item = String;
    type IntoIter = IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
