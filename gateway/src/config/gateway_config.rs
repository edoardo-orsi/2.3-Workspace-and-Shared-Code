use crate::{ServerConfig, ServicesConfig};
use common::{load_and_validate, AppError, BaseConfig};
use serde::Deserialize;
use std::path::Path;
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Validate)]
pub struct GatewayConfig {
    #[validate(nested)]
    pub base: BaseConfig,

    #[validate(nested)]
    pub server: ServerConfig,

    #[validate(nested)]
    pub services: ServicesConfig,
}

impl GatewayConfig {
    pub fn load_from_path(base_dir: &Path) -> Result<Self, AppError> {
        let config = load_and_validate(base_dir)?;
        Ok(config)
    }
}
