use std::path::Path;
use figment::Figment;
use figment::providers::{Env, Format, Yaml};
use serde::{Deserialize, Serialize};
use crate::app_error::AppError;
use crate::configuration::logging::LoggingConfig;
use crate::configuration::service::ServiceConfig;

/// Shared configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub service: ServiceConfig,
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

        // let _ = config.logging.init();

        Ok(config)
    }

    pub fn init_logging(&self) -> Result<(), AppError> {
        let _ = self.logging.init();
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), AppError> {
        if self.service.name.is_empty() {
            return Err(AppError::ConfigError("Service name cannot be empty".into()));
        }

        if self.service.version.is_empty() {
            return Err(AppError::ConfigError("Service version cannot be empty".into()));
        }

        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(AppError::ConfigError(format!(
                "Invalid log level: {}",
                self.logging.level
            )));
        }

        Ok(())
    }
}