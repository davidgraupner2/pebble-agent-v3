use crate::query::{FilterCondition, FilterOperator};

pub(crate) mod agent_identity;
pub(crate) mod agent_jwt;
pub(crate) mod cache;
pub(crate) mod connection_stats;
pub(crate) mod connection_strings;
pub(crate) mod encrytion_keys;
pub(crate) mod events;
pub(crate) mod function_hashes;
pub(crate) mod properties;
pub(crate) mod registration;
pub(crate) mod registration_challenge;
pub(crate) mod secrets;
pub(crate) mod tags;
// Re-exports are NOT public - only accessible via AppContainer

pub fn enforce_tenant_filter(
    filters: &[FilterCondition],
    registration_id: impl Into<String>,
) -> Vec<FilterCondition> {
    let tenant_id = registration_id.into();

    let mut normalized: Vec<FilterCondition> = filters
        .iter()
        .filter(|f| !f.field.eq_ignore_ascii_case("registration_id"))
        .cloned()
        .collect();

    normalized.push(FilterCondition {
        field: "registration_id".to_string(),
        operator: FilterOperator::Eq,
        value: tenant_id,
    });

    normalized
}
