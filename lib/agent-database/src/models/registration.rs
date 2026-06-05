use crate::schema::registration;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, PartialEq, Serialize, Debug, Clone)]
#[diesel(table_name = registration)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Registration {
    pub id: i32,
    pub agent_id: String,
    pub jti: String,
    pub expires_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize, Clone)]
#[diesel(table_name = registration)]
pub struct NewRegistration {
    pub agent_id: String,
    pub jti: String,
    pub source: String,
    pub expires_at: Option<NaiveDateTime>
}

impl NewRegistration {
    pub fn new(agent_id: String, source: String, expires_at: Option<NaiveDateTime>) -> Self {
        Self{
            agent_id,
            jti: Uuid::new_v4().to_string(), 
            source,
            expires_at
        }
    }
}

