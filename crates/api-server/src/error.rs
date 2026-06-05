use capitalize::Capitalize;
use salvo::prelude::*;
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Serialize)]
struct JsonErrorResponse {
    brief: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<String>,
    code: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    name: String,
}

#[derive(Error, Debug, ToSchema)]
pub enum ApiError {
    // #[error("Error bootstrapping the agent: {0}")]
    // BootStrapError(String),
    #[error("Database Error: {0}")]
    DatabaseError(String),

    #[error("{0}")]
    ConnectionStringValidationError(String),

    #[error("{0}")]
    WebSocketError(String),

    #[error("Data Access Error: {0}")]
    DataAccessError(String),

    #[error("server error: {0}")]
    ServerError(String),

    // #[error("Invalid input: {0}")]
    // InvalidInputError(String),
    #[error("JSON Error: {0}")]
    JSONError(String),

    #[error("Config Error: {0}")]
    ConfigError(String),

    #[error("Encryption not initialized: {0}")]
    EncryptionNotInitializedError(String),

    #[error("There is already a {0} with the name of {1}")]
    DuplicateNameError(String, String),

    #[error("No valid encryption key found with id of {0}")]
    InvalidEncryptionKey(i32),

    #[error("{0}")]
    BadRequest(String),

    // #[error("{0}")]
    // SerializationError(String),
    #[error("{0}")]
    DeSerializationError(String),

    #[error("{0}")]
    DecryptionFailed(String),

    #[error("{0}")]
    NotFoundError(String),

    #[error("{0}")]
    SalvoError(#[from] salvo::prelude::StatusError),
}

impl From<agent_database::DatabaseError> for ApiError {
    fn from(e: agent_database::DatabaseError) -> Self {
        ApiError::DatabaseError(e.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::JSONError(e.to_string())
    }
}

#[async_trait]
impl Writer for ApiError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        let status_error = match &self {
            ApiError::DatabaseError(error) => {
                error!("Database error - {}", error);
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);

                StatusError::internal_server_error()
                    .brief("Internal Database Error")
                    .detail(format!("{}", Capitalize::capitalize_first_only(error),))
            }
            ApiError::ConnectionStringValidationError(error) => {
                error!("Connection String could not be validated - {}", error);

                StatusError::bad_request()
                    .brief("Invalid Connection String")
                    .detail(format!(
                        "Connection String could not be validated - {}",
                        Capitalize::capitalize_first_only(error)
                    ))
            }
            ApiError::WebSocketError(error) => {
                error!("Web Socket Error - {}", error);

                StatusError::internal_server_error()
                    .brief("Web Socket Error")
                    .detail(format!("{}", Capitalize::capitalize_first_only(error)))
            }
            ApiError::DataAccessError(error) => {
                error!("Data access error - {}", error);

                StatusError::internal_server_error()
                    .brief("Data access error")
                    .detail(format!("{}", Capitalize::capitalize_first_only(error)))
            }
            ApiError::ServerError(error) => {
                error!("Server Error - {}", error);

                StatusError::internal_server_error()
                    .brief("Server Error")
                    .detail(format!("{}", Capitalize::capitalize_first_only(error)))
            }
            ApiError::JSONError(error) => {
                error!("JSON Error - {}", error);

                StatusError::bad_request()
                    .brief("JSON Parsing Error")
                    .detail(format!("{}", Capitalize::capitalize_first_only(error)))
            }
            ApiError::ConfigError(error) => {
                error!("Configuration Error - {}", error);

                StatusError::internal_server_error()
                    .brief("Configuration Error")
                    .detail(format!("{}", Capitalize::capitalize_first_only(error)))
            }
            ApiError::EncryptionNotInitializedError(error) => {
                error!("Encryption Subsystem could not be initialised - {}", error);

                StatusError::internal_server_error()
                    .brief("Encryption Subsystem Error")
                    .detail(format!("{}", Capitalize::capitalize_first_only(error)))
            }
            ApiError::DuplicateNameError(object, name) => {
                error!(
                    "Duplicate name - Already a {} with a name of {}",
                    object, name
                );
                StatusError::bad_request()
                    .brief("Duplication Error")
                    .detail(format!(
                    "This operation would result in a duplicate name. {} already has a name of {}",
                    Capitalize::capitalize_first_only(object),
                    name,))
            }
            ApiError::InvalidEncryptionKey(key) => {
                error!("Invalid Encryption Key - {}", key);

                StatusError::bad_request()
                    .brief("Encryption Key Error")
                    .detail(format!("Invalid encryption key. {}", key))
            }
            ApiError::BadRequest(error) => {
                error!("Bad Request - {}", error);

                StatusError::bad_request()
                    .brief("Bad request")
                    .detail(format!("Bad Request - {}", error))
            }
            ApiError::DeSerializationError(error) => {
                error!("Deserializing error: {}", error);

                StatusError::bad_request()
                    .brief("Deserialisation Error")
                    .detail(format!("{}", error))
            }
            ApiError::DecryptionFailed(error) => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);

                StatusError::internal_server_error()
                    .brief("Decryption Failed")
                    .detail(format!("{}", error))
            }
            ApiError::NotFoundError(error) => StatusError::bad_request()
                .brief("Not found")
                .detail(format!("{}", error)),
            ApiError::SalvoError(error) => {
                error!("Salvo Error: {}", error);
                StatusError::internal_server_error()
                    .brief("Server Error")
                    .detail(format!("{}", error))
            }
        };

        res.status_code(status_error.code);

        if let Some(cause) = status_error.cause {
            res.render(Json(JsonErrorResponse {
                brief: status_error.brief,
                cause: Some(cause.to_string()),
                code: status_error.code.as_u16(),
                detail: status_error.detail,
                name: status_error.name,
            }))
        } else {
            res.render(Json(JsonErrorResponse {
                brief: status_error.brief,
                cause: None,
                code: status_error.code.as_u16(),
                detail: status_error.detail,
                name: status_error.name,
            }))
        }
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;

use salvo::http::{StatusCode, StatusError};
use salvo::oapi::{self, EndpointOutRegister, ToSchema};

impl EndpointOutRegister for ApiError {
    fn register(components: &mut oapi::Components, operation: &mut oapi::Operation) {
        operation.responses.insert(
            StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            oapi::Response::new("Internal server error")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::NOT_FOUND.as_str(),
            oapi::Response::new("Not found")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::BAD_REQUEST.as_str(),
            oapi::Response::new("Bad request")
                .add_content("application/json", StatusError::to_schema(components)),
        );
    }
}
