use crate::AppError;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use serde::de::DeserializeOwned;
use std::path::Path;
use validator::Validate;

/// A generic loader that merges base YAML, env-specific YAML, and Env Vars.
pub fn load_and_validate<T>(base_dir: &Path) -> Result<T, AppError>
where
    T: DeserializeOwned + Validate,
{
    let env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());

    let base_config_path = base_dir.join("config.yaml");
    let env_config_path = base_dir.join(format!("config.{}.yaml", env));

    // Initialize Figment
    let figment = Figment::new()
        .merge(Yaml::file(base_config_path))
        .merge(Yaml::file(env_config_path))
        .merge(Env::prefixed("APP__").split("__"));

    // 1. Extract the data
    let config: T = figment
        .extract()
        .map_err(|e| AppError::ConfigError(e.to_string()))?;

    // 2. Run the Validator (Nested validation works here if #[validate(nested)] is used)
    config
        .validate()
        .map_err(|e| AppError::ConfigError(format!("Validation failed: {}", e)))?;

    Ok(config)
}
