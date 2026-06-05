// Default Property names used for API configuration
pub const PROPERTY_API_PORT: &str = "api::port";
pub const PROPERTY_API_LOGGING_FORMAT: &str = "api::logging.format";
pub const PROPERTY_API_LOGGING_LEVEL: &str = "api::logging.level";
pub const PROPERTY_API_ACTIVE_ENCRYPTION_KEY: &str = "api::encryption.key";
pub const PROPERTY_API_ENCRYPTION_TOKEN_COUNT: &str = "api::encryption.tokens.count";
pub const PROPERTY_API_LOGGING_OUTPUT: &str = "api::logging.output";
pub const PROPERTY_API_SYMMETRICAL_TOKENS: &str = "api::tokens.list";
pub const PROPERTY_API_CS_VALIDATE: &str = "api::connection_string::validate";
pub const PROPERTY_API_CS_VALIDATE_AUDIENCE: &str = "api::connection_string::validate_audience";
pub const PROPERTY_API_CS_VALIDATE_SCHEME: &str = "api::connection_string::validate_scheme";
pub const PROPERTY_API_CS_VALIDATE_SECRET_PARAMETER_NAME: &str =
    "api::connection_string::validate_secret_parameter_name";
pub const PROPERTY_API_CS_EXTRACT_ENVIRONMENT: &str = "api::connection_string::extract_environment";
pub const PROPERTY_API_JWT_EXPIRY_MINUTES: &str = "api::jwt_expiry_minutes";
pub const PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS: &str =
    "api::agent_registration_expiry_seconds";

// Property defaults, if properties not  loaded into the database
pub const DEFAULT_PROPERTY_API_PORT: i32 = 8174;
pub const DEFAULT_PROPERTY_API_CS_VALIDATE_SCHEME: &str = "wss";
pub const DEFAULT_PROPERTY_API_CS_VALIDATE: bool = true;
pub const DEFAULT_PROPERTY_API_CS_VALIDATE_AUDIENCE: &str = "thepebble.io";
pub const DEFAULT_PROPERTY_API_CS_VALIDATE_SECRET_PARAMETER_NAME: &str = "s";
pub const DEFAULT_PROPERTY_API_CS_EXTRACT_ENVIRONMENT: &str = "sub:.*:0";
pub const DEFAULT_PROPERTY_API_JWT_EXPIRY_MINUTES: i32 = 0;
pub const DEFAULT_PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS: i32 = 120;
