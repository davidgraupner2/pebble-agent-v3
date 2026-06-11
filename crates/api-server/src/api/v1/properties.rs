use crate::api::extensions::DepotExt;
use crate::error::{ApiError, Result};
use agent_database::DatabaseError::ValidationErrorCannotChangeConfigType;
use agent_database::{Property, PropertyValue, RepositoryGetSet, TypedProperty};
use salvo::oapi::extract::JsonBody;
use salvo::oapi::extract::PathParam;
use salvo::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use tracing::error;

#[derive(Debug, Deserialize, ToSchema, Clone)]
/// Request payload used for property create/update operations.
///
/// The value is accepted as arbitrary JSON and normalized into a typed
/// `PropertyValue` before being persisted.
pub struct UpsertPropertyRequest {
    pub name: String,
    pub description: Option<String>,
    pub value: Value,
}

/// Convert inbound JSON into the repository's strongly typed property variant.
///
/// Normalization rules:
/// - `bool` -> `PropertyValue::Bool`
/// - `string` -> `PropertyValue::String`
/// - integer numbers fitting `i32` -> `PropertyValue::Int`
/// - everything else -> `PropertyValue::Json`
fn infer_property_value(value: Value) -> PropertyValue {
    match value {
        Value::Bool(v) => PropertyValue::Bool(v),
        Value::String(v) => PropertyValue::String(v),
        Value::Number(v) => {
            if let Some(as_i64) = v.as_i64() {
                if let Ok(as_i32) = i32::try_from(as_i64) {
                    return PropertyValue::Int(as_i32);
                }
            }
            PropertyValue::Json(Value::Number(v))
        }
        other => PropertyValue::Json(other),
    }
}

/// Build all V1 property routes.
///
/// Mounted endpoints:
/// - `GET /property/{name}`
/// - `GET /properties`
/// - `POST /property`
/// - `POST /properties`
/// - `DELETE /property/{name}`
/// - `DELETE /properties`
pub fn properties_router() -> Router {
    Router::new()
        .push(Router::with_path("property/{name}").get(get_property))
        .push(Router::with_path("properties").get(get_properties))
        .push(Router::with_path("property").post(add_property))
        .push(Router::with_path("properties").post(add_properties))
        .push(Router::with_path("property/{name}").delete(delete_property))
        .push(Router::with_path("properties").delete(delete_properties))
}

/// Retrieve a persisted property for the authenticated agent.
///
/// Why this endpoint exists:
/// - Agents need to read effective runtime settings from durable storage.
/// - Property lookup is scoped to the caller identity, preventing cross-agent reads.
///
/// How it works:
/// - The caller supplies the property `name` in the route path.
/// - The API resolves the caller's `registration_id` from the validated bearer token.
/// - The property repository performs a scoped lookup by `(name, registration_id)`.
///
/// Response behavior:
/// - `200`: property found and returned as a typed payload.
/// - `404`: no property exists for that name within the caller scope.
/// - `500`: unexpected storage or server failure.
///
/// Security notes:
/// - Requires a valid bearer JWT.
/// - Data access is tenant-scoped by registration identity.
#[endpoint(security(("bearer_token"=[])),tags("Properties"), status_codes(200, 401, 404, 500))]
async fn get_property(depot: &mut Depot, name: PathParam<String>) -> Result<Json<TypedProperty>> {
    let property_repo = depot.repositories()?.properties_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();
    let property_name = name.clone().to_string();

    match property_repo.get(
        &mut db_connection,
        property_name.clone(),
        registration_id.clone(),
    ) {
        Ok(Some(property)) => Ok(Json(property)),
        Ok(None) => Err(ApiError::NotFoundError(format!(
            "Property {} not found",
            name
        ))),
        Err(error) => {
            error!(errorMsg=%error, name=property_name,registration_id=registration_id,"Error retrieving property from database");
            Err(ApiError::ServerError(error.to_string()))
        }
    }
}

/// Retrieve persisted properties for the authenticated agent.
///
/// Why this endpoint exists:
/// - Agents need to read effective runtime settings from durable storage.
/// - Property lookup is scoped to the caller identity, preventing cross-agent reads.
///
/// How it works:
/// - The caller requests all scoped properties
/// - The API resolves the caller's `registration_id` from the validated bearer token.
/// - The property repository performs a scoped lookup by `(registration_id)`.
///
/// Response behavior:
/// - `200`: properties found and returned as a array of typed payloads.
/// - `404`: no properties exists for that caller scope.
/// - `500`: unexpected storage or server failure.
///
/// Security notes:
/// - Requires a valid bearer JWT.
/// - Data access is tenant-scoped by registration identity.
#[endpoint(security(("bearer_token"=[])),tags("Properties"), status_codes(200, 401, 404, 500))]
async fn get_properties(depot: &mut Depot) -> Result<Json<Vec<TypedProperty>>> {
    let property_repo = depot.repositories()?.properties_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();

    match property_repo.get_all(&mut db_connection, registration_id.clone()) {
        Ok(properties) => {
            if properties.is_empty() {
                Err(ApiError::NotFoundError("No Properties found".to_string()))
            } else {
                Ok(Json(properties))
            }
        }
        Err(error) => {
            error!(errorMsg=%error,registration_id=registration_id,"Error retrieving properties from database");
            Err(ApiError::ServerError(error.to_string()))
        }
    }
}

/// Create or update a single property for the authenticated agent.
///
/// Why this endpoint exists:
/// - Agents need to persist runtime settings that may change over time.
/// - Upsert semantics avoid a read-before-write roundtrip for known keys.
///
/// How it works:
/// - The caller sends `name`, optional `description`, and a JSON `value`.
/// - The API resolves the caller's `registration_id` from the validated bearer token.
/// - The incoming JSON value is normalized into `PropertyValue` and stored as an upsert.
///
/// Response behavior:
/// - `200`: property persisted and returned.
/// - `400`: malformed request body.
/// - `404`: not used by this endpoint.
/// - `401`: missing or invalid bearer token.
/// - `500`: unexpected storage or server failure.
///
/// Security notes:
/// - Requires a valid bearer JWT.
/// - Data mutation is tenant-scoped by registration identity.
#[endpoint(security(("bearer_token"=[])),tags("Properties"), status_codes(200, 400, 401, 500))]
async fn add_property(
    depot: &mut Depot,
    payload: JsonBody<UpsertPropertyRequest>,
) -> Result<Json<TypedProperty>> {
    let property_repo = depot.repositories()?.properties_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();
    let typed_value = infer_property_value(payload.value.clone());

    let property_to_add = typed_value.to_new_property(
        payload.name.clone(),
        registration_id.clone(),
        payload.description.clone(),
        "api".to_string(),
    );

    match property_repo.set(&mut db_connection, property_to_add) {
        Ok(property) => Ok(Json(property)),
        Err(error) => {
            error!(errorMsg=%error,registration_id=registration_id,name=%payload.name,"Error adding property to database");
            match error {
                ValidationErrorCannotChangeConfigType(..) => {
                    Err(ApiError::BadRequest(error.to_string()))
                }
                _ => Err(ApiError::ServerError(error.to_string())),
            }
        }
    }
}

/// Create or update multiple properties for the authenticated agent.
///
/// Why this endpoint exists:
/// - Agents often bootstrap or refresh multiple settings in one operation.
/// - Batch upsert reduces request overhead and keeps updates consistent.
///
/// How it works:
/// - The caller sends an array of upsert payloads.
/// - The API resolves the caller's `registration_id` from the validated bearer token.
/// - Each item is normalized into `PropertyValue` and persisted in a single batch upsert.
///
/// Response behavior:
/// - `200`: properties persisted and returned.
/// - `400`: malformed request body.
/// - `404`: not used by this endpoint.
/// - `401`: missing or invalid bearer token.
/// - `500`: unexpected storage or server failure.
///
/// Security notes:
/// - Requires a valid bearer JWT.
/// - Data mutation is tenant-scoped by registration identity.
#[endpoint(security(("bearer_token"=[])),tags("Properties"), status_codes(200, 400, 401, 500))]
async fn add_properties(
    depot: &mut Depot,
    payload: JsonBody<Vec<UpsertPropertyRequest>>,
) -> Result<Json<Vec<TypedProperty>>> {
    let property_repo = depot.repositories()?.properties_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();

    let mut property_records: Vec<Property> = Vec::new();

    for (_idx, item) in payload.clone().into_iter().enumerate() {
        let typed_value = infer_property_value(item.value);
        let property: Property = typed_value.to_new_property(
            item.name,
            registration_id.clone(),
            item.description,
            "api".to_string(),
        );
        property_records.push(property);
    }

    match property_repo.set_many(&mut db_connection, property_records) {
        Ok(properties) => Ok(Json(properties)),
        Err(error) => {
            error!(errorMsg=%error,registration_id=registration_id,"Error adding properties to database");
            Err(ApiError::ServerError(error.to_string()))
        }
    }
}

/// Delete one scoped property for the authenticated agent.
///
/// Why this endpoint exists:
/// - Agents need to remove obsolete or invalid runtime settings.
/// - Deletes must be scoped so one agent cannot remove another agent's data.
///
/// How it works:
/// - The caller supplies the property `name` in the route path.
/// - The API resolves the caller's `registration_id` from the validated bearer token.
/// - The repository deletes by `(name, registration_id)` and reports affected rows.
///
/// Response behavior:
/// - `200`: property deleted.
/// - `401`: missing or invalid bearer token.
/// - `404`: property not found for this agent.
/// - `500`: unexpected storage or server failure.
///
/// Security notes:
/// - Requires a valid bearer JWT.
/// - Data mutation is tenant-scoped by registration identity.
#[endpoint(security(("bearer_token"=[])),tags("Properties"), status_codes(200, 401, 404, 500))]
async fn delete_property(depot: &mut Depot, name: PathParam<String>) -> Result<String> {
    let property_repo = depot.repositories()?.properties_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();
    let property_name = name.clone().to_string();

    match property_repo.delete(
        &mut db_connection,
        property_name.clone(),
        registration_id.clone(),
    ) {
        Ok(record_count) => {
            if record_count == 0 {
                Err(ApiError::NotFoundError(format!(
                    "Property with name '{}' not found!",
                    name
                )))
            } else {
                Ok(format!("{} Property record deleted", record_count))
            }
        }
        Err(error) => {
            error!(errorMsg=%error, name=property_name,registration_id=registration_id,"Error deleting  property from database");
            Err(ApiError::ServerError(error.to_string()))
        }
    }
}

/// Delete all scoped properties for the authenticated agent.
///
/// Why this endpoint exists:
/// - Agents may need a full property reset during reconfiguration.
/// - Bulk delete must remain scoped to the authenticated caller.
///
/// How it works:
/// - The caller invokes the endpoint without a body.
/// - The API resolves the caller's `registration_id` from the validated bearer token.
/// - The repository deletes all records for that registration scope and reports row count.
///
/// Response behavior:
/// - `200`: one or more properties deleted.
/// - `401`: missing or invalid bearer token.
/// - `404`: no properties found to delete.
/// - `500`: unexpected storage or server failure.
///
/// Security notes:
/// - Requires a valid bearer JWT.
/// - Data mutation is tenant-scoped by registration identity.
#[endpoint(security(("bearer_token"=[])),tags("Properties"), status_codes(200, 401, 404, 500))]
async fn delete_properties(depot: &mut Depot) -> Result<String> {
    let property_repo = depot.repositories()?.properties_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();

    match property_repo.delete_all(&mut db_connection, registration_id.clone()) {
        Ok(record_count) => {
            if record_count == 0 {
                Err(ApiError::NotFoundError(format!(
                    "No properties found to delete!",
                )))
            } else {
                Ok(format!("{} Property record deleted", record_count))
            }
        }
        Err(error) => {
            error!(errorMsg=%error, registration_id=registration_id,"Error deleting  property from database");
            Err(ApiError::ServerError(error.to_string()))
        }
    }
}
