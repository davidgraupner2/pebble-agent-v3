use diesel::result::{DatabaseErrorKind, Error as DieselError};
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Error executing query: {0}")]
    QueryError(String),

    #[error("Error saving data to the database: {0}")]
    SaveError(String),

    #[error("Error getting data from the database: {0}")]
    RetrieveError(String),

    #[error("Requested record not found")]
    NotFoundError,

    #[error("Rollback Error: Rollback:{0}, Commit:{1}")]
    RollbackError(String, String),

    #[error("Transaction rolled back")]
    RolledBack,

    #[error("Already in transaction")]
    AlreadyInTransaction,

    #[error("Not in a transaction")]
    NotInTransaction,

    #[error("Broken transaction manager")]
    BrokeTransactionManager,

    #[error("Unique key or index violation: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    UniqueKeyViolation(String, String, String),

    #[error("Foreign key violation: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    ForeignKeyViolation(String, String, String),

    #[error("Unable to send command: Message: '{0}' Hint: '{1}'")]
    SendCommandError(String, String),

    #[error("Read only transaction encountered: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    ReadOnlyTransactionError(String, String, String),

    #[error("Error saving to database: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    SerializationError(String, String, String),

    #[error("Restriction violation:: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    RestrictViolationError(String, String, String),

    #[error("Null violation: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    NotNullError(String, String, String),

    #[error("Check constraint violation: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    CheckConstraintViolationError(String, String, String),

    #[error("Exclusion constraint violation: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    ExclusionConstraintViolationError(String, String, String),

    #[error("No database connection: Message: '{0}' Hint: '{1}'")]
    ClosedConnectionError(String, String),

    #[error("Generic Database error: Table: '{0}' Message: '{1}' Hint: '{2}'")]
    GenericDatabaseError(String, String, String),

    #[error("Unable to convert {0} '{1}' to {2}")]
    ConversionError(String, String, String),

    #[error("Connection error: {0}")]
    Connection(#[from] diesel::ConnectionError),

    #[error("Connection error: {0}")]
    PoolError(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Could not parse integer value: {0}")]
    ParseIntError(#[from] ParseIntError),

    #[error("Invalid Input: {0}")]
    InvalidInput(String),

    #[error("Cannot delete records that have not been disabled: [{0}]")]
    DeleteNotDisabled(String),

    #[error("Invalid Filter: Filter condition of '{0}' cannot be applied to field of {1}")]
    InvalidFilter(String, String),

    #[error(
        "Invalid encryption key provided: Encryption key with id '{0}' either does not exist or is not enabled"
    )]
    InvalidEncryptionKey(i32),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("{0}")]
    PublicKeyEncodingError(String),
}

impl From<DieselError> for DatabaseError {
    fn from(err: DieselError) -> Self {
        match err {
            DieselError::DatabaseError(database_error_kind, database_error_information) => {
                match database_error_kind {
                    DatabaseErrorKind::UniqueViolation => DatabaseError::UniqueKeyViolation(
                        database_error_information
                            .table_name()
                            .unwrap_or("")
                            .to_string(),
                        database_error_information.message().to_string(),
                        database_error_information.hint().unwrap_or("").to_string(),
                    ),
                    DatabaseErrorKind::ForeignKeyViolation => DatabaseError::ForeignKeyViolation(
                        database_error_information
                            .table_name()
                            .unwrap_or("")
                            .to_string(),
                        database_error_information.message().to_string(),
                        database_error_information.hint().unwrap_or("").to_string(),
                    ),
                    DatabaseErrorKind::UnableToSendCommand => DatabaseError::SendCommandError(
                        database_error_information.message().to_string(),
                        database_error_information.hint().unwrap_or("").to_string(),
                    ),
                    DatabaseErrorKind::SerializationFailure => DatabaseError::SerializationError(
                        database_error_information
                            .table_name()
                            .unwrap_or("")
                            .to_string(),
                        database_error_information.message().to_string(),
                        database_error_information.hint().unwrap_or("").to_string(),
                    ),
                    DatabaseErrorKind::ReadOnlyTransaction => {
                        DatabaseError::ReadOnlyTransactionError(
                            database_error_information
                                .table_name()
                                .unwrap_or("")
                                .to_string(),
                            database_error_information.message().to_string(),
                            database_error_information.hint().unwrap_or("").to_string(),
                        )
                    }
                    DatabaseErrorKind::RestrictViolation => DatabaseError::RestrictViolationError(
                        database_error_information
                            .table_name()
                            .unwrap_or("")
                            .to_string(),
                        database_error_information.message().to_string(),
                        database_error_information.hint().unwrap_or("").to_string(),
                    ),
                    DatabaseErrorKind::NotNullViolation => DatabaseError::NotNullError(
                        database_error_information
                            .table_name()
                            .unwrap_or("")
                            .to_string(),
                        database_error_information.message().to_string(),
                        database_error_information.hint().unwrap_or("").to_string(),
                    ),
                    DatabaseErrorKind::CheckViolation => {
                        DatabaseError::CheckConstraintViolationError(
                            database_error_information
                                .table_name()
                                .unwrap_or("")
                                .to_string(),
                            database_error_information.message().to_string(),
                            database_error_information.hint().unwrap_or("").to_string(),
                        )
                    }
                    DatabaseErrorKind::ExclusionViolation => {
                        DatabaseError::ExclusionConstraintViolationError(
                            database_error_information
                                .table_name()
                                .unwrap_or("")
                                .to_string(),
                            database_error_information.message().to_string(),
                            database_error_information.hint().unwrap_or("").to_string(),
                        )
                    }
                    DatabaseErrorKind::ClosedConnection => DatabaseError::ClosedConnectionError(
                        database_error_information.message().to_string(),
                        database_error_information.hint().unwrap_or("").to_string(),
                    ),
                    _ => DatabaseError::GenericDatabaseError(
                        database_error_information
                            .table_name()
                            .unwrap_or("")
                            .to_string(),
                        database_error_information.message().to_string(),
                        database_error_information.hint().unwrap_or("").to_string(),
                    ),
                }
            }
            DieselError::NotFound => DatabaseError::NotFoundError,
            DieselError::QueryBuilderError(error) => DatabaseError::QueryError(error.to_string()),
            DieselError::DeserializationError(error) => {
                DatabaseError::RetrieveError(error.to_string())
            }
            DieselError::SerializationError(error) => DatabaseError::SaveError(error.to_string()),
            DieselError::RollbackErrorOnCommit {
                rollback_error,
                commit_error,
            } => DatabaseError::RollbackError(rollback_error.to_string(), commit_error.to_string()),
            DieselError::RollbackTransaction => DatabaseError::RolledBack,
            DieselError::AlreadyInTransaction => DatabaseError::AlreadyInTransaction,
            DieselError::NotInTransaction => DatabaseError::NotInTransaction,
            DieselError::BrokenTransactionManager => DatabaseError::BrokeTransactionManager,
            _ => todo!(),
        }
    }
}

pub type Result<T> = std::result::Result<T, DatabaseError>;
