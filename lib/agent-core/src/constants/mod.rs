pub mod folders;

use crate::constants::folders::Folders;
use base64::prelude::*;
use machineid_rs::{Encryption, HWIDComponent, IdBuilder};
use std::collections::BTreeMap;
use std::env;
use std::env::home_dir;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::RwLock;
use sysinfo::System;

// Global constants used by all consumers
pub const DATABASE_NAME: &str = "agent.db";
// pub const REGISTRATION_KEY_FILE: &str = "agent.key.json";
pub const AGENT_SOURCE: &str = "agent";
pub const ACTOR_API_NAME: &str = "API Server";
pub const ACTOR_API_DISCOVERY_FILE: &str = "api-discovery.json";
// pub const ACTOR_FUNCTION_EXECUTOR_NAME: &str = "Function Executor";
// pub const ACTOR_CONNECTION_MANAGER_NAME: &str = "Connection Manager";
// pub const ACTOR_DATABASE_EVENT_BUS_NAME: &str = "Database Event Bus";
pub const ENCRYPTION_PREFIX: &str = "enc!";
pub const ENCRYPTION_DATABASE_BOX_NAME: &str = "database";
pub const ENCRYPTION_CONNECTION_STRING_TABLE_ENCRYPTOR: &str = "Connection Strings Table";
pub const ENCRYPTION_CONNECTION_STRING_TABLE_ENCRYPTOR_CONTEXT: &str = "pR98+dr=Fi3l";
pub const DECRYPTION_KEY_HEADER_NAME: &str = "x-decryption-key";
pub const DECRYPTION_KEY_NOTES: &str =  "Please store the encryption key details away in a secure location. \
             You will need to provide the 'id' or 'name' when encrypting records. \
             To unencrypt records you will need to provide the private key as the 'x-encryption-key' header value";

// Global var to store runtime constants
static RUNTIME_CONSTANTS: OnceLock<RuntimeConstants> = OnceLock::new();

#[derive(Debug)]
pub struct RuntimeConstants {
    app_name: Box<str>,
    version: Box<str>,
    machine_name: Box<str>,
    host_name: Box<str>,
    exe_name: Box<str>,
    id: Box<str>,
    api_id: Box<str>,
    folders: Box<Folders>,
    api_discovery_file: Box<String>,
    registration_key_file: PathBuf,
    pub files: Arc<RwLock<BTreeMap<String, PathBuf>>>,
}

impl RuntimeConstants {
    /// Initialize the runtime properties globally. Must be called once at application startup.
    ///
    /// # Panics
    /// Panics if called more than once.
    pub fn init(app_name: &str) {
        RUNTIME_CONSTANTS
            .set(RuntimeConstants::new(app_name))
            .expect("RUNTIME_PROPERTIES already initialised");
    }

    /// Initialize runtime properties with a custom base directory for folders.
    ///
    /// Useful for tests: callers can pass a temporary directory to avoid
    /// creating folders under system paths like `/var/log`.
    pub fn init_with_base(app_name: &str, base: &std::path::Path) {
        RUNTIME_CONSTANTS
            .set(RuntimeConstants::new_with_base(app_name, base))
            .expect("RUNTIME_PROPERTIES already initialised");
    }

    /// Get a reference to the global runtime properties.
    ///
    /// # Panics
    /// Panics if `RuntimeProperties::init()` hasn't been called yet.
    pub fn global() -> &'static RuntimeConstants {
        RUNTIME_CONSTANTS
            .get()
            .expect("RUNTIME_PROPERTIES not initialized. Call RuntimeProperties::init() first.")
    }

    pub fn new(app_name: &str) -> Self {
        let version = option_env!("CARGO_PKG_VERSION")
            .unwrap_or("0.0.0")
            .to_string()
            .into_boxed_str();
        let name = System::name().unwrap_or_default().into_boxed_str();
        let host_name = System::host_name().unwrap_or_default().into_boxed_str();
        let exe_name = std::env::current_exe()
            .unwrap()
            .with_extension("")
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or("default")
            .to_string()
            .into_boxed_str();
        let id = runtime_id().into_boxed_str();
        let api_id = runtime_api_id().into_boxed_str();
        let folders = Box::new(Folders::new(app_name.to_lowercase().as_str()).ensure_exists());
        let api_discovery_file_name = folders
            .clone()
            .discovery()
            .join(ACTOR_API_DISCOVERY_FILE)
            .to_string_lossy()
            .to_string();
        let base64_encoded_registration_key_file_name = BASE64_URL_SAFE.encode(id.as_bytes());
        let registration_key_file = folders.clone().supplementary_files().join(format!(
            "{}_key.json",
            base64_encoded_registration_key_file_name
        ));

        Self {
            app_name: app_name.into(),
            version,
            machine_name: name,
            host_name,
            exe_name,
            id,
            api_id,
            folders,
            files: Arc::new(RwLock::new(BTreeMap::new())),
            api_discovery_file: Box::new(api_discovery_file_name),
            registration_key_file: registration_key_file,
        }
    }

    /// Create a `RuntimeProperties` instance but use an explicit base path for
    /// folder creation. This mirrors `init_with_base` but returns the instance
    /// so callers (tests) can inspect it without initializing the global.
    pub fn new_with_base(app_name: &str, base: &std::path::Path) -> Self {
        let version = option_env!("CARGO_PKG_VERSION")
            .unwrap_or("0.0.0")
            .to_string()
            .into_boxed_str();
        let name = System::name().unwrap_or_default().into_boxed_str();
        let host_name = System::host_name().unwrap_or_default().into_boxed_str();
        let exe_name = std::env::current_exe()
            .unwrap()
            .with_extension("")
            .file_name()
            .unwrap()
            .to_str()
            .unwrap_or("default")
            .to_string()
            .into_boxed_str();
        let id = runtime_id().into_boxed_str();
        let api_id = runtime_api_id().into_boxed_str();
        let folders = Box::new(
            Folders::new_with_base(app_name.to_lowercase().as_str(), base).ensure_exists(),
        );
        let api_discovery_file_name = folders
            .clone()
            .discovery()
            .join(ACTOR_API_DISCOVERY_FILE)
            .to_string_lossy()
            .to_string();
        let base64_encoded_registration_key_file_name = BASE64_URL_SAFE.encode(id.as_bytes());
        let registration_key_file = folders.clone().supplementary_files().join(format!(
            "{}_key.json",
            base64_encoded_registration_key_file_name
        ));

        Self {
            app_name: app_name.into(),
            version,
            machine_name: name,
            host_name,
            exe_name,
            id,
            api_id,
            folders,
            files: Arc::new(RwLock::new(BTreeMap::new())),
            api_discovery_file: Box::new(api_discovery_file_name),
            registration_key_file: registration_key_file,
        }
    }

    // Accessor methods
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn name(&self) -> &str {
        &self.machine_name
    }
    pub fn host_name(&self) -> &str {
        &self.host_name
    }
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn api_id(&self) -> &str {
        &self.api_id
    }

    pub fn exe_name(&self) -> &str {
        &self.exe_name
    }
    pub fn folders(&self) -> &Folders {
        &self.folders
    }
    pub fn app_name(&self) -> &str {
        &self.app_name
    }
    pub fn api_discovery_file_name(&self) -> &str {
        &self.api_discovery_file
    }

    pub fn database_file_name(&self) -> String {
        self.folders
            .supplementary_files()
            .join(DATABASE_NAME)
            .as_os_str()
            .to_string_lossy()
            .into_owned()
    }

    pub fn agent_registration_key_file(&self) -> &PathBuf {
        &self.registration_key_file
    }

    /// Register (insert or replace) a name -> path entry.
    pub fn register_file(&self, name: impl Into<String>, path: impl Into<PathBuf>) {
        let mut map = self.files.write().unwrap();
        map.insert(name.into(), path.into());
    }

    /// Retrieve a cloned PathBuf for a registered name, if present.
    pub fn get_file(&self, name: &str) -> Option<PathBuf> {
        let map = self.files.read().unwrap();
        map.get(name).cloned()
    }
}

fn runtime_id() -> String {
    // Get the current home directory
    // - We use this as a value for generating a unique id, this allows us to have different ids for agents in different directories
    let current_dir = env::current_dir()
        .unwrap()
        .clone()
        .into_os_string()
        .into_string()
        .unwrap()
        .clone()
        .replace("/", "")
        .replace("\\", "");

    // let current_dir = home_dir()
    //     .unwrap()
    //     .into_os_string()
    //     .into_string()
    //     .unwrap()
    //     .clone()
    //     .replace("/", "")
    //     .replace("\\", "");
    let current_dir_static: &'static str = Box::leak(current_dir.into_boxed_str());

    let mut hardware_id_builder = IdBuilder::new(Encryption::SHA256);
    hardware_id_builder
        .add_component(HWIDComponent::SystemID)
        .add_component(HWIDComponent::CPUID)
        .add_component(HWIDComponent::OSName)
        .add_component(HWIDComponent::FileToken(current_dir_static));

    hardware_id_builder.build("id").unwrap()
}

fn runtime_api_id() -> String {
    // Get the current home directory
    // - We use this as a value for generating a unique id, this allows us to have different ids for API's (if we needed) in different directories

    let current_dir = env::current_dir()
        .unwrap()
        .clone()
        .into_os_string()
        .into_string()
        .unwrap()
        .clone()
        .replace("/", "")
        .replace("\\", "");

    // let current_dir = home_dir()
    //     .unwrap()
    //     .into_os_string()
    //     .into_string()
    //     .unwrap()
    //     .clone()
    //     .replace("/", "")
    //     .replace("\\", "");
    let current_dir_static: &'static str = Box::leak(current_dir.into_boxed_str());

    let mut hardware_id_builder = IdBuilder::new(Encryption::SHA256);
    hardware_id_builder
        .add_component(HWIDComponent::SystemID)
        .add_component(HWIDComponent::CPUID)
        .add_component(HWIDComponent::OSName)
        .add_component(HWIDComponent::FileToken(current_dir_static))
        .add_component(HWIDComponent::FileToken(ACTOR_API_NAME));

    format!("api::{}", hardware_id_builder.build("api_id").unwrap())
}
