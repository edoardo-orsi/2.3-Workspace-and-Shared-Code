use crate::app_error::AppError;
use crate::configuration::logging::LoggingConfig;
use crate::configuration::service::ServiceConfig;
use crate::ServiceConfigLogic;
use serde::Deserialize;
use std::path::Path;
use validator::Validate;

/// Shared configuration
#[derive(Debug, Clone, PartialEq, Eq, Validate, Deserialize)]
pub struct BaseConfig {
    #[validate(nested)]
    pub service: ServiceConfig,

    #[validate(nested)]
    pub logging: LoggingConfig,
}
impl ServiceConfigLogic for BaseConfig {}

impl BaseConfig {
    /// Load configuration from multiple sources
    pub fn load_from_path(base_dir: &Path) -> Result<Self, AppError> {
        let config = Self::load_and_validate(base_dir)?;
        Ok(config)
    }

    pub fn init_logging(&self) -> Result<(), AppError> {
        let _ = self.logging.init();
        Ok(())
    }
}
