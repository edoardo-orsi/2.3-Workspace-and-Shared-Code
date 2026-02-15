use std::path::Path;
use serde::{Deserialize, Serialize};
use common::{BaseConfig, CommonError, ServerConfig, ServiceConfigLogic};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct GreeterConfig {
    #[validate(nested)]
    pub base: BaseConfig,
    
    #[validate(nested)]
    pub server: ServerConfig,
}

impl ServiceConfigLogic for GreeterConfig {}

impl GreeterConfig {
    pub fn load_from_path(base_dir: &Path) -> Result<Self, CommonError> {
        let config = Self::load_and_validate(base_dir)?;
        Ok(config)
    }
}