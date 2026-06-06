use crate::schema::agent_jwt;
use diesel::backend::Backend; // Added to access Backend::RawValue
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow, Serialize, Deserialize)]
#[diesel(sql_type = Text)]
pub enum AgentJwtStatus {
    Active,
    Inactive,
    Expired,
}

impl ToSql<Text, Sqlite> for AgentJwtStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let s = match self {
            AgentJwtStatus::Active => "active",
            AgentJwtStatus::Inactive => "inactive",
            AgentJwtStatus::Expired => "expired",
        };
        out.set_value(s.to_string());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for AgentJwtStatus {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        // Fixed: Using <Sqlite as Backend>::RawValue directly clears the deprecation warning
        let s = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        match s.as_str() {
            "active" => Ok(AgentJwtStatus::Active),
            "inactive" => Ok(AgentJwtStatus::Inactive),
            "expired" => Ok(AgentJwtStatus::Expired),
            _ => Err("Unknown AgentJwtStatus value".into()),
        }
    }
}

// 1. Queryable model (for reading from the database)
#[derive(Debug, Clone, Queryable, Selectable, AsChangeset)]
#[diesel(table_name = agent_jwt)]
#[diesel(check_for_backend(Sqlite))]
pub struct AgentJwt {
    pub id: i32,
    pub registration_id: String,
    pub jti: String,
    pub status: AgentJwtStatus,
}

// 2. Insertable model (for creating new rows; id is omitted)
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = agent_jwt)]
pub struct NewAgentJwt {
    pub registration_id: String,
    pub jti: String,
    pub status: AgentJwtStatus,
}
