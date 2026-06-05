use crate::schema::events;
use chrono::NaiveDateTime;
use diesel::AsExpression;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::Display;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow, Serialize, Deserialize, Display,
)]
#[diesel(sql_type = crate::schema::sql_types::EventStatus)]
pub enum EventStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl Default for EventStatus {
    fn default() -> Self {
        EventStatus::Pending
    }
}

// Serialize: Rust -> Database
impl ToSql<crate::schema::sql_types::EventStatus, Sqlite> for EventStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let value = match self {
            EventStatus::Pending => "pending",
            EventStatus::InProgress => "in_progress",
            EventStatus::Completed => "completed",
            EventStatus::Failed => "failed",
            EventStatus::Cancelled => "cancelled",
        };
        out.set_value(value);
        Ok(IsNull::No)
    }
}

// Deserialize: Database -> Rust
impl FromSql<crate::schema::sql_types::EventStatus, Sqlite> for EventStatus {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        match value.as_str() {
            "pending" => Ok(EventStatus::Pending),
            "in_progress" => Ok(EventStatus::InProgress),
            "completed" => Ok(EventStatus::Completed),
            "failed" => Ok(EventStatus::Failed),
            "cancelled" => Ok(EventStatus::Cancelled),
            _ => Err(format!("Unrecognized enum variant: {}", value).into()),
        }
    }
}

impl std::str::FromStr for EventStatus {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(EventStatus::Pending),
            "in_progress" => Ok(EventStatus::InProgress),
            "completed" => Ok(EventStatus::Completed),
            "failed" => Ok(EventStatus::Failed),
            "cancelled" => Ok(EventStatus::Cancelled),
            _ => Err("Unknown status".into()),
        }
    }
}

#[derive(Queryable, Selectable, Serialize, Debug)]
#[diesel(table_name = events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Event {
    pub id: i32,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    #[serde(serialize_with = "serialize_payload_as_json")]
    pub payload: String,
    pub metadata: Option<String>,
    pub status: EventStatus,
    pub retry_count: i32,
    pub processed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

fn serialize_payload_as_json<S>(payload: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Try to parse as JSON
    match serde_json::from_str::<Value>(payload) {
        Ok(json_value) => json_value.serialize(serializer),
        Err(_) => payload.serialize(serializer), // Fall back to string
    }
}

#[derive(Insertable, Debug, Deserialize)]
#[diesel(table_name = events)]
pub struct NewEvent {
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub payload: String,
    pub metadata: String,
    #[diesel(column_name = status)]
    pub status: Option<EventStatus>,
}

#[derive(Insertable, Debug, AsChangeset, Identifiable, Deserialize)]
#[diesel(table_name = events)]
pub struct UpdateEvent {
    pub id: i32,
    pub status: Option<EventStatus>,
    pub retry_count: Option<i32>,
    pub metadata: Option<String>,
}
