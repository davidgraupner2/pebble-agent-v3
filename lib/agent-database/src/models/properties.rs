use crate::schema::properties;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Queryable, Selectable, Serialize, Debug)]
#[diesel(table_name = properties)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PropertyRecord {
    pub id: i32,
    pub agent_uuid: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub source: String,
    pub description: Option<String>,
    pub value_int: Option<i32>,
    pub value_string: Option<String>,
    pub value_bool: Option<i32>, // 0 or 1
    pub value_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = properties)]
pub struct Property {
    pub agent_uuid: Option<String>,
    pub name: String,
    pub type_: String,
    pub description: Option<String>,
    pub source: String,
    pub value_int: Option<i32>,
    pub value_string: Option<String>,
    pub value_bool: Option<i32>,
    pub value_json: Option<String>,
}

impl Display for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value_display = match self.type_.as_str() {
            "int" => self.value_int.map(|v| v.to_string()).unwrap_or_default(),
            "string" => self.value_string.clone().unwrap_or_default(),
            "bool" => self
                .value_bool
                .map(|v| if v != 0 { "true" } else { "false" }.to_string())
                .unwrap_or_default(),
            "json" => self.value_json.clone().unwrap_or_default(),
            _ => "unknown".to_string(),
        };

        write!(
            f,
            "Property '{:?}-{}' [{}]: {}{}",
            self.agent_uuid,
            self.name,
            self.type_,
            value_display,
            self.description
                .as_ref()
                .map(|d| format!(" ({})", d))
                .unwrap_or_default()
        )
    }
}

#[derive(Insertable, Debug, AsChangeset, Identifiable)]
#[diesel(table_name = properties)]
pub struct UpdateProperty {
    pub id: i32,
    pub agent_uuid: Option<String>,
    pub name: String,
    pub type_: String,
    pub description: Option<String>,
    pub source: String,
    pub value_int: Option<i32>,
    pub value_string: Option<String>,
    pub value_bool: Option<i32>,
    pub value_json: Option<String>,
}

/// Typed enum for API usage
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(tag = "type", content = "value")]
pub enum PropertyValue {
    #[serde(rename = "int")]
    Int(i32),
    #[serde(rename = "string")]
    String(String),
    #[serde(rename = "bool")]
    Bool(bool),
    #[serde(rename = "json")]
    Json(serde_json::Value),
}

/// Clean API response struct
#[derive(Serialize, Debug, ToSchema)]
pub struct TypedProperty {
    pub id: i32,
    #[cfg_attr(not(debug_assertions), serde(skip_serializing))]
    pub agent_uuid: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub source: String,
    #[serde(flatten)]
    pub value: PropertyValue,
    pub created_at: String,
    pub updated_at: String,
}

// Helper to convert DB row to typed value
impl PropertyRecord {
    pub fn value(&self) -> Option<PropertyValue> {
        match self.type_.as_str() {
            "int" => self.value_int.map(PropertyValue::Int),
            "string" => self.value_string.clone().map(PropertyValue::String),
            "bool" => self.value_bool.map(|v| PropertyValue::Bool(v != 0)),
            "json" => self
                .value_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok().map(PropertyValue::Json)),
            _ => None,
        }
    }

    /// Convert to API-friendly format
    pub fn to_typed(&self) -> Option<TypedProperty> {
        self.value().map(|v| TypedProperty {
            id: self.id,
            agent_uuid: self.agent_uuid.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            source: self.source.clone(),
            value: v,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        })
    }
}

impl PropertyValue {
    /// Create NewProperty from typed value
    pub fn to_new_property(
        self,
        name: String,
        agent_uuid: Option<String>,
        description: Option<String>,
        source: String,
    ) -> Property {
        match self {
            PropertyValue::Int(v) => Property {
                name,
                agent_uuid,
                type_: "int".to_string(),
                description,
                source,
                value_int: Some(v),
                value_string: None,
                value_bool: None,
                value_json: None,
            },
            PropertyValue::String(v) => Property {
                name,
                agent_uuid,
                type_: "string".to_string(),
                description,
                source,
                value_int: None,
                value_string: Some(v),
                value_bool: None,
                value_json: None,
            },
            PropertyValue::Bool(v) => Property {
                name,
                agent_uuid,
                type_: "bool".to_string(),
                description,
                source,
                value_int: None,
                value_string: None,
                value_bool: Some(if v { 1 } else { 0 }),
                value_json: None,
            },
            PropertyValue::Json(v) => Property {
                name,
                agent_uuid,
                type_: "json".to_string(),
                description,
                source,
                value_int: None,
                value_string: None,
                value_bool: None,
                value_json: Some(serde_json::to_string(&v).unwrap()),
            },
        }
    }
}
