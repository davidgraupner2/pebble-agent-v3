use crate::schema::connection_stats;
use chrono::NaiveDateTime;
use diesel::AsExpression;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow, Serialize, Deserialize)]
#[diesel(sql_type = crate::schema::sql_types::ConnectionStatsStatus)]
pub enum ConnectionStatsStatus {
    Connected,
    Disconnected,
}

impl Default for ConnectionStatsStatus {
    fn default() -> Self {
        ConnectionStatsStatus::Connected
    }
}

// Serialize: Rust -> Database
impl ToSql<crate::schema::sql_types::ConnectionStatsStatus, Sqlite> for ConnectionStatsStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let value = match self {
            ConnectionStatsStatus::Connected => "connected",
            ConnectionStatsStatus::Disconnected => "disconnected",
        };
        out.set_value(value);
        Ok(IsNull::No)
    }
}

// // Deserialize: Database -> Rust
impl FromSql<crate::schema::sql_types::ConnectionStatsStatus, Sqlite> for ConnectionStatsStatus {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        match value.as_str() {
            "connected" => Ok(ConnectionStatsStatus::Connected),
            "disconnected" => Ok(ConnectionStatsStatus::Disconnected),
            _ => Err(format!("Unrecognized enum variant: {}", value).into()),
        }
    }
}

impl std::str::FromStr for ConnectionStatsStatus {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "connected" => Ok(ConnectionStatsStatus::Connected),
            "disconnected" => Ok(ConnectionStatsStatus::Disconnected),
            _ => Err("Unknown status".into()),
        }
    }
}

#[derive(Queryable, Selectable, Serialize, Debug)]
#[diesel(table_name = connection_stats)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ConnectionStats {
    pub id: i32,
    pub endpoint: String,
    pub status: ConnectionStatsStatus,
    pub connected_at: NaiveDateTime,
    pub disconnected_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Deserialize, Clone)]
#[diesel(table_name = connection_stats)]
pub struct NewConnectionStats {
    pub endpoint: String,
}

#[derive(Queryable, Identifiable, AsChangeset, Deserialize, Clone)]
#[diesel(table_name = connection_stats)]
pub struct UpdateConnectionStats {
    pub id: i32,
    pub status: ConnectionStatsStatus,
}
