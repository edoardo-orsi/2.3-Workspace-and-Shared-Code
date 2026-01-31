use crate::{CommonError, PathExt};
use figment::providers::{Env, Format, Serialized, Yaml};
use figment::Figment;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::warn;
use validator::Validate;

pub trait ServiceConfigLogic: DeserializeOwned + Validate {
    /// A generic loader that merges base YAML, env-specific YAML, and Env Vars.
    /// Loads the configuration from the filesystem and environment variables.
    fn load_and_validate(base_dir: &Path) -> Result<Self, CommonError> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());
        let service_prefix = Self::get_env_prefix();
        let secrets_dir = Self::get_secrets_dir();

        let base_config_path = base_dir.join("config.yaml");
        let env_config_path = base_dir.join(format!("config.{}.yaml", env));

        if !base_config_path.exists() {
            return Err(CommonError::ConfigError(format!(
                "Config file not found at: {}",
                base_config_path
                    .canonicalize()
                    .unwrap_or(base_config_path)
                    .display()
            )));
        }

        let mut figment = Figment::new()
            .merge(Yaml::file(base_config_path))
            .merge(Yaml::file(env_config_path))
            .merge(Env::prefixed("APP__").split("__"))
            .merge(Env::prefixed(&format!("{}_", service_prefix)).split("__"));

        // Inject secrets if the directory exists
        if secrets_dir.exists() {
            if secrets_dir.has_files() {
                let secrets = Self::load_secrets_from_dir(&secrets_dir);
                figment = figment.merge(Serialized::defaults(secrets));
            } else {
                warn!(
                    "Secrets directory does not have files: {}",
                    secrets_dir.display()
                );
            }
        } else {
            warn!(
                "Secrets directory does not exists at {}",
                secrets_dir.display()
            );
        }

        let config: Self = figment
            .extract()
            .map_err(|e| CommonError::ConfigError(e.to_string()))?;

        config.validate().map_err(|e| {
            for (field_name, field_errors) in e.field_errors() {
                for error in field_errors {
                    // Priority 1: Specific Secret failures
                    if error.code == "missing_secret" {
                        return CommonError::MissingSecret(field_name.to_string());
                    }

                    // Priority 2: General missing fields
                    if error.code == "required" {
                        return CommonError::ConfigError(format!(
                            "Field '{}' is missing in configuration",
                            field_name
                        ));
                    }
                }
            }
            CommonError::ConfigError(format!("Validation failed: {}", e))
        })?;

        Ok(config)
    }

    /// Detects the service name from the running binary filename
    fn get_service_name() -> String {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "APP".to_string())
    }

    /// Formats the service name for Env Var use (e.g., "SERVICE_ENGINE").
    fn get_env_prefix() -> String {
        Self::get_service_name().to_uppercase().replace('-', "_")
    }

    /// The directory where secrets are mounted (K8s/Docker standard)
    fn get_secrets_dir() -> std::path::PathBuf {
        std::env::var("APP_SECRETS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/run/secrets"))
    }

    /// Checks if a directory exists and contains at least one file.
    fn has_files(dir: &Path) -> bool {
        dir.is_dir()
            && fs::read_dir(dir)
                .map(|mut i| i.next().is_some()) // Check if at least one entry exists
                .unwrap_or(false)
    }

    /// Automated Path Discovery
    fn auto_base_dir() -> std::path::PathBuf {
        let raw_name = Self::get_service_name();
        let current_working_directory = std::env::current_dir().unwrap_or_default();

        if current_working_directory.join("config.yaml").exists() {
            current_working_directory
        } else if current_working_directory
            .join(&raw_name)
            .join("config.yaml")
            .exists()
        {
            current_working_directory.join(&raw_name)
        } else {
            current_working_directory
        }
    }

    /// The new automated loader
    fn load_auto() -> Result<Self, CommonError> {
        let base_dir = Self::auto_base_dir();
        Self::load_and_validate(&base_dir)
    }

    /// Helper to read K8s/Docker secret files into a Map
    fn load_secrets_from_dir(dir: &Path) -> HashMap<String, String> {
        let mut secrets = HashMap::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(key) = path.file_name().and_then(|n| n.to_str()) {
                        if let Ok(value) = fs::read_to_string(&path) {
                            // Map the filename (key) to its content (value)
                            // We trim to handle trailing newlines common in K8s mounts
                            secrets.insert(key.to_lowercase(), value.trim().to_string());
                        }
                    }
                }
            }
        }
        secrets
    }
}
