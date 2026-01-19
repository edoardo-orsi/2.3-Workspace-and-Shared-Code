use crate::app_error::AppError;
use crate::configuration::logging::LoggingConfig;
use crate::configuration::service::ServiceConfig;
use figment::providers::{Env, Format, Yaml};
use figment::Figment;
use serde::{Deserialize, Serialize};
use std::path::Path;
use validator::Validate;

/// Shared configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct Config {
    #[validate(nested)]
    pub service: ServiceConfig,

    #[validate(nested)]
    pub logging: LoggingConfig,
}

impl Config {
    /// Load configuration from multiple sources
    pub fn load_from_path(base_dir: &Path) -> Result<Self, AppError> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());

        let base_config_path = base_dir.join("config.yaml");
        let env_config_path = base_dir.join(format!("config.{}.yaml", env));

        let config: Config = Figment::new()
            // Start with base config
            .merge(Yaml::file(base_config_path))
            // Merge environment-specific config
            .merge(Yaml::file(env_config_path))
            // Environment variables override everything
            // Format: APP__SERVER__PORT=8080 -> server.port = 8080
            .merge(Env::prefixed("APP__").split("__"))
            .extract()
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        // Run Validator
        config
            .validate()
            .map_err(|e| AppError::ConfigError(format!("Validation failed: {}", e)))?;

        Ok(config)
    }

    pub fn init_logging(&self) -> Result<(), AppError> {
        let _ = self.logging.init();
        Ok(())
    }
}
