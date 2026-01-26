use crate::AppError;
use figment::providers::{Env, Format, Yaml};
use figment::Figment;
use serde::de::DeserializeOwned;
use std::path::Path;
use validator::Validate;

pub trait ServiceConfigLogic: DeserializeOwned + Validate {
    /// A generic loader that merges base YAML, env-specific YAML, and Env Vars.
    /// Loads the configuration from the filesystem and environment variables.
    fn load_and_validate(base_dir: &Path) -> Result<Self, AppError> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());

        let base_config_path = base_dir.join("config.yaml");
        let env_config_path = base_dir.join(format!("config.{}.yaml", env));

        let figment = Figment::new()
            .merge(Yaml::file(base_config_path))
            .merge(Yaml::file(env_config_path))
            .merge(Env::prefixed("APP__").split("__"));

        let config: Self = figment
            .extract()
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        config
            .validate()
            .map_err(|e| AppError::ConfigError(format!("Validation failed: {}", e)))?;

        Ok(config)
    }
}
