use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum SecretValue {
    #[serde(rename = "amazon_s3")]
    AmazonS3 {
        client_id: String,
        client_secret: String,
        region: String,
        bucket_name: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "amazon_web_services")]
    AmazonWebServices {
        regional_endpoint_code: String,
        access_key_id: String,
        secret_access_key: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "azure_blob_storage")]
    AzureBlobStorage {
        connection_string: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "azure_cloud")]
    AzureCloud {
        tenant_id: String,
        client_id: String,
        client_secret: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "basic")]
    Basic {
        username: String,
        password: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "basic_api")]
    BasicApi {
        url: String,
        username: String,
        password: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "basic_api_oauth")]
    BasicApiOauth {
        url: String,
        client_id: String,
        client_secret: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "basic_api_token")]
    BasicApiToken {
        url: String,
        token: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "generic")]
    Generic { content: String },
    #[serde(rename = "intune")]
    Intune {
        tenant_id: String,
        client_id: String,
        client_secret: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "mobileiron")]
    MobileIron {
        url: String,
        username: String,
        password: String,
        space: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "servicenow")]
    ServiceNow {
        url: String,
        username: String,
        password: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "snmpv2")]
    SNMPv1v2 {
        community: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "snmpv3")]
    SNMPv3 {
        username: String,
        authentication_type: Option<String>,
        authentication_password: Option<String>,
        privacy_type: Option<String>,
        privacy_password: Option<String>,
        additional_config: Option<String>,
    },
    #[serde(rename = "ssh")]
    SSH {
        username: String,
        password: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "ssh_private_key")]
    SSHPrivateKey {
        username: String,
        private_key: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "windows_domain")]
    WindowsDomain {
        domain: String,
        username: String,
        password: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "xurrent_oauth")]
    XurrentOAUTH {
        url: String,
        account: String,
        client_id: String,
        client_secret: String,
        additional_config: Option<String>,
    },
    #[serde(rename = "xurrent_pat")]
    XurrentPAT {
        url: String,
        account: String,
        personal_access_token: String,
        additional_config: Option<String>,
    },
}

impl SecretValue {
    /// Serialize to JSON string for storage
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json_string(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn secret_type(&self) -> &'static str {
        match self {
            SecretValue::Generic { .. } => "generic",
            SecretValue::SNMPv1v2 { .. } => "snmpv1v2",
            SecretValue::SNMPv3 { .. } => "snmpv3",
            SecretValue::AmazonS3 { .. } => "amazon_s3",
            SecretValue::AmazonWebServices { .. } => "amazon_web_services",
            SecretValue::AzureBlobStorage { .. } => "azure_blob_storage",
            SecretValue::AzureCloud { .. } => "azure_cloud",
            SecretValue::Basic { .. } => "basic",
            SecretValue::BasicApi { .. } => "basic_api",
            SecretValue::BasicApiOauth { .. } => "basic_api_oauth",
            SecretValue::BasicApiToken { .. } => "basic_api_token",
            SecretValue::Intune { .. } => "intune",
            SecretValue::MobileIron { .. } => "mobileiron",
            SecretValue::ServiceNow { .. } => "servicenow",
            SecretValue::SSH { .. } => "ssh",
            SecretValue::SSHPrivateKey { .. } => "ssh_private_key",
            SecretValue::WindowsDomain { .. } => "windows_domain",
            SecretValue::XurrentOAUTH { .. } => "xurrent_oauth",
            SecretValue::XurrentPAT { .. } => "xurrent_pat",
        }
    }
}

// ============= Secret Type Info =============
#[derive(Serialize, Clone)]
pub struct SecretTypeInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub required_fields: Vec<&'static str>,
    pub optional_fields: Vec<&'static str>,
}

pub fn get_secret_type_info() -> Vec<SecretTypeInfo> {
    vec![
        SecretTypeInfo {
            name: "amazon_s3",
            description: "Amazon S3 credentials",
            required_fields: vec!["client_id", "client_secret", "region", "bucket_name"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "amazon_web_services",
            description: "Amazon Web Services credentials",
            required_fields: vec![
                "regional_endpoint_code",
                "access_key_id",
                "secret_access_key",
            ],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "azure_blob_storage",
            description: "Azure Blob Storage connection",
            required_fields: vec!["connection_string"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "azure_cloud",
            description: "Azure Cloud credentials",
            required_fields: vec!["tenant_id", "client_id", "client_secret"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "basic",
            description: "Basic username/password",
            required_fields: vec!["username", "password"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "basic_api",
            description: "Basic API authentication",
            required_fields: vec!["url", "username", "password"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "basic_api_oauth",
            description: "API OAuth authentication",
            required_fields: vec!["url", "client_id", "client_secret"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "basic_api_token",
            description: "API token authentication",
            required_fields: vec!["url", "token"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "generic",
            description: "Generic secret content",
            required_fields: vec!["content"],
            optional_fields: vec![],
        },
        SecretTypeInfo {
            name: "intune",
            description: "Microsoft Intune credentials",
            required_fields: vec!["tenant_id", "client_id", "client_secret"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "mobileiron",
            description: "MobileIron credentials",
            required_fields: vec!["url", "username", "password", "space"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "servicenow",
            description: "ServiceNow credentials",
            required_fields: vec!["url", "username", "password"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "snmpv1v2",
            description: "SNMPv1/v2 community string",
            required_fields: vec!["community"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "snmpv3",
            description: "SNMPv3 credentials",
            required_fields: vec!["username"],
            optional_fields: vec![
                "authentication_type",
                "authentication_password",
                "privacy_type",
                "privacy_password",
                "additional_config",
            ],
        },
        SecretTypeInfo {
            name: "ssh",
            description: "SSH password authentication",
            required_fields: vec!["username", "password"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "ssh_private_key",
            description: "SSH private key authentication",
            required_fields: vec!["username", "private_key"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "windows_domain",
            description: "Windows domain credentials",
            required_fields: vec!["domain", "username", "password"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "xurrent_oauth",
            description: "Xurrent OAuth credentials",
            required_fields: vec!["url", "account", "client_id", "client_secret"],
            optional_fields: vec!["additional_config"],
        },
        SecretTypeInfo {
            name: "xurrent_pat",
            description: "Xurrent Personal Access Token",
            required_fields: vec!["url", "account", "personal_access_token"],
            optional_fields: vec!["additional_config"],
        },
    ]
}
