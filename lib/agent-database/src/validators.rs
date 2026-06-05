use crate::SecretValue;
use crate::errors::DatabaseError;

pub fn validate_secret_value(secret: &SecretValue) -> Result<(), DatabaseError> {
    match secret {
        SecretValue::AmazonS3 {
            client_id,
            client_secret,
            region,
            bucket_name,
            ..
        } => {
            if client_id.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AmazonS3: client_id is required".to_string(),
                ));
            }
            if client_secret.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AmazonS3: client_secret is required".to_string(),
                ));
            }
            if region.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AmazonS3: region is required".to_string(),
                ));
            }
            if bucket_name.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AmazonS3: bucket_name is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::AmazonWebServices {
            regional_endpoint_code,
            access_key_id,
            secret_access_key,
            ..
        } => {
            if regional_endpoint_code.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AmazonWebServices: regional_endpoint_code is required".to_string(),
                ));
            }
            if access_key_id.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AmazonWebServices: access_key_id is required".to_string(),
                ));
            }
            if secret_access_key.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AmazonWebServices: secret_access_key is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::AzureBlobStorage {
            connection_string, ..
        } => {
            if connection_string.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AzureBlobStorage: connection_string is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::AzureCloud {
            tenant_id,
            client_id,
            client_secret,
            ..
        } => {
            if tenant_id.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AzureCloud: tenant_id is required".to_string(),
                ));
            }
            if client_id.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AzureCloud: client_id is required".to_string(),
                ));
            }
            if client_secret.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "AzureCloud: client_secret is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::Basic {
            username, password, ..
        } => {
            if username.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "Basic: username is required".to_string(),
                ));
            }
            if password.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "Basic: password is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::BasicApi {
            url,
            username,
            password,
            ..
        } => {
            if url.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "BasicApi: url is required".to_string(),
                ));
            }
            if username.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "BasicApi: username is required".to_string(),
                ));
            }
            if password.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "BasicApi: password is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::BasicApiOauth {
            url,
            client_id,
            client_secret,
            ..
        } => {
            if url.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "BasicApiOauth: url is required".to_string(),
                ));
            }
            if client_id.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "BasicApiOauth: client_id is required".to_string(),
                ));
            }
            if client_secret.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "BasicApiOauth: client_secret is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::BasicApiToken { url, token, .. } => {
            if url.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "BasicApiToken: url is required".to_string(),
                ));
            }
            if token.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "BasicApiToken: token is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::Generic { content } => {
            if content.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "Generic: content is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::Intune {
            tenant_id,
            client_id,
            client_secret,
            ..
        } => {
            if tenant_id.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "Intune: tenant_id is required".to_string(),
                ));
            }
            if client_id.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "Intune: client_id is required".to_string(),
                ));
            }
            if client_secret.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "Intune: client_secret is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::MobileIron {
            url,
            username,
            password,
            space,
            ..
        } => {
            if url.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "MobileIron: url is required".to_string(),
                ));
            }
            if username.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "MobileIron: username is required".to_string(),
                ));
            }
            if password.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "MobileIron: password is required".to_string(),
                ));
            }
            if space.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "MobileIron: space is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::ServiceNow {
            url,
            username,
            password,
            ..
        } => {
            if url.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "ServiceNow: url is required".to_string(),
                ));
            }
            if username.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "ServiceNow: username is required".to_string(),
                ));
            }
            if password.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "ServiceNow: password is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::SNMPv1v2 { community, .. } => {
            if community.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "SNMPv1v2: community is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::SNMPv3 { username, .. } => {
            if username.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "SNMPv3: username is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::SSH {
            username, password, ..
        } => {
            if username.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "SSH: username is required".to_string(),
                ));
            }
            if password.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "SSH: password is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::SSHPrivateKey {
            username,
            private_key,
            ..
        } => {
            if username.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "SSHPrivateKey: username is required".to_string(),
                ));
            }
            if private_key.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "SSHPrivateKey: private_key is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::WindowsDomain {
            domain,
            username,
            password,
            ..
        } => {
            if domain.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "WindowsDomain: domain is required".to_string(),
                ));
            }
            if username.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "WindowsDomain: username is required".to_string(),
                ));
            }
            if password.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "WindowsDomain: password is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::XurrentOAUTH {
            url,
            account,
            client_id,
            client_secret,
            ..
        } => {
            if url.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "XurrentOAUTH: url is required".to_string(),
                ));
            }
            if account.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "XurrentOAUTH: account is required".to_string(),
                ));
            }
            if client_id.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "XurrentOAUTH: client_id is required".to_string(),
                ));
            }
            if client_secret.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "XurrentOAUTH: client_secret is required".to_string(),
                ));
            }
            Ok(())
        }
        SecretValue::XurrentPAT {
            url,
            account,
            personal_access_token,
            ..
        } => {
            if url.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "XurrentPAT: url is required".to_string(),
                ));
            }
            if account.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "XurrentPAT: account is required".to_string(),
                ));
            }
            if personal_access_token.is_empty() {
                return Err(DatabaseError::ValidationError(
                    "XurrentPAT: personal_access_token is required".to_string(),
                ));
            }
            Ok(())
        }
    }
}
