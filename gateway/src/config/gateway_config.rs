use crate::{GatewayError, ServerConfig, ServicesConfig};
use common::{BaseConfig, ServiceConfigLogic};
use serde::Deserialize;
use std::path::Path;
use validator::Validate;

// TODO mandatory secret
// #[derive(Deserialize, Validate)]
// pub struct DatabaseConfig {
//     #[validate(required(code = "missing_secret"))]
//     pub password: Option<String>,
// }
// #[validate(required)] is typically used on Option<T> types to ensure they aren't None.
// (code = "missing_secret") Overrides the default code and let us have a ley in the validation to derive the proper error message
// To make certain secrets mandatory it is possible to use the #[validate(required)] attribute
// to the sensitive fields in your Rust structs. If the secret isn't found in YAML, Env, or the Secret Dir,
// the config.validate() call at the end of the trait will catch it and prevent the service from running without a password.

// Since GatewayConfig contains BaseConfig as a nested field,
// we should only instantiate GatewayConfig.
//
// When Figment loads the YAML, it maps the base: key in the file directly into the
// pub base: BaseConfig field of the gateway struct.
//
// Because we use #[validate(nested)] and the ServiceConfigLogic trait on GatewayConfig,
// the single call to load_and_validate handles the entire tree:
//  - Parsing: It fills base, server, and services.
//  - Validation: It triggers validation for the gateway, which cascades down to BaseConfig, LoggingConfig, etc.

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Validate)]
pub struct GatewayConfig {
    #[validate(nested)]
    pub base: BaseConfig,

    #[validate(nested)]
    pub server: ServerConfig,

    #[validate(nested)]
    pub services: ServicesConfig,
}

impl ServiceConfigLogic for GatewayConfig {}

impl GatewayConfig {
    pub fn load_from_path(base_dir: &Path) -> Result<Self, GatewayError> {
        let config = Self::load_and_validate(base_dir)?;
        Ok(config)
    }
}
