use crate::Tags;
use crate::schema::connection_strings;
use chrono::NaiveDateTime;
use core::fmt;
use diesel::AsExpression;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use serde::{Deserialize, Serialize};

// Flatten tag names to just Vec<String>
pub(crate) fn serialize_tag_names<S>(tags: &[Tags], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
    names.serialize(serializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow, Serialize)]
#[diesel(sql_type = crate::schema::sql_types::ConnectionStringStatus)]
pub enum ConnectionStringStatus {
    Pending,
    InUse,
}

impl Default for ConnectionStringStatus {
    fn default() -> Self {
        ConnectionStringStatus::Pending
    }
}

// Serialize: Rust -> Database
impl ToSql<crate::schema::sql_types::ConnectionStringStatus, Sqlite> for ConnectionStringStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let value = match self {
            ConnectionStringStatus::Pending => "pending",
            ConnectionStringStatus::InUse => "in_use",
        };
        out.set_value(value);
        Ok(IsNull::No)
    }
}

// Deserialize: Database -> Rust
impl FromSql<crate::schema::sql_types::ConnectionStringStatus, Sqlite> for ConnectionStringStatus {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        match value.as_str() {
            "pending" => Ok(ConnectionStringStatus::Pending),
            "in_use" => Ok(ConnectionStringStatus::InUse),
            _ => Err(format!("Unrecognized enum variant: {}", value).into()),
        }
    }
}

impl std::str::FromStr for ConnectionStringStatus {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(ConnectionStringStatus::Pending),
            "in_use" => Ok(ConnectionStringStatus::InUse),
            _ => Err("Unknown status".into()),
        }
    }
}

impl std::fmt::Display for ConnectionStringStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnectionStringStatus::Pending => write!(f, "pending"),
            ConnectionStringStatus::InUse => write!(f, "in_use"),
        }
    }
}

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = connection_strings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ConnectionString {
    pub id: i32,
    pub value: String,
    pub description: Option<String>,
    pub source: String,
    pub status: ConnectionStringStatus,
    pub environment: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize, Serialize)]
#[diesel(table_name = connection_strings)]
pub struct NewConnectionString {
    pub value: String,
    pub source: String,
    pub description: Option<String>,
    pub environment: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ApiConnectionString {
    pub value: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ApiConnectionStringWithEnvironment {
    pub value: String,
    pub description: Option<String>,
    pub environment: Option<String>,
}

#[derive(Queryable, Identifiable, AsChangeset, Serialize, Clone)]
#[diesel(table_name = connection_strings)]
pub struct UpdateConnectionString {
    pub id: i32,
    pub status: ConnectionStringStatus,
}

impl ApiConnectionStringWithEnvironment {
    pub fn to_connection_string(
        &self,
        value: String,
        description: Option<String>,
        environment: Option<String>,
    ) -> NewConnectionString {
        NewConnectionString {
            value,
            source: "api".to_string(),
            description,
            environment,
        }
    }
}
