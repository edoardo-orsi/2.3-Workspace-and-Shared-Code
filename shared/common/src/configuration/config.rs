use figment::Figment;
use figment::providers::{Env, Format, Yaml};
use serde::{Deserialize, Serialize};
use crate::app_error::AppError;
use crate::configuration::logging::LoggingConfig;
use crate::configuration::service::ServiceConfig;

/// Shared configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub service: ServiceConfig,
    pub logging: LoggingConfig,
}

impl Config {
    /// Load configuration from multiple sources
    pub fn load() -> Result<Self, AppError> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());

        let config: Config = Figment::new()
            // Start with base config
            .merge(Yaml::file("config.yaml"))

            // Merge environment-specific config
            .merge(Yaml::file(format!("config.{}.yaml", env)))

            // Environment variables override everything
            // Format: APP__SERVER__PORT=8080 -> server.port = 8080
            .merge(Env::prefixed("APP__").split("__"))
            
            .extract()
            .map_err(|e| AppError::ConfigError(e.to_string()))?;

        config.logging.init();

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), AppError> {
        if self.service.name.is_empty() {
            return Err(AppError::ConfigError("Service name cannot be empty".into()));
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