use crate::CommonError;
use figment::providers::{Env, Format, Yaml};
use figment::Figment;
use serde::de::DeserializeOwned;
use std::path::Path;
use validator::Validate;

pub trait ServiceConfigLogic: DeserializeOwned + Validate {
    /// A generic loader that merges base YAML, env-specific YAML, and Env Vars.
    /// Loads the configuration from the filesystem and environment variables.
    fn load_and_validate(base_dir: &Path) -> Result<Self, CommonError> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());
        let service_prefix = Self::get_env_prefix();

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

        let figment = Figment::new()
            .merge(Yaml::file(base_config_path))
            .merge(Yaml::file(env_config_path))
            .merge(Env::prefixed("APP__").split("__"))
            .merge(Env::prefixed(&format!("{}_", service_prefix)).split("__"));

        let config: Self = figment
            .extract()
            .map_err(|e| CommonError::ConfigError(e.to_string()))?;

        config
            .validate()
            .map_err(|e| CommonError::ConfigError(format!("Validation failed: {}", e)))?;

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
}
