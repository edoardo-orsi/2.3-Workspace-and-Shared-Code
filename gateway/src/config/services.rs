use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct ServicesConfig {
    #[validate(nested)]
    pub greeter: ServiceEndpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct ServiceEndpoint {
    #[validate(url)]
    pub url: String,

    #[serde(default)]
    pub timeout_ms: u64,

    #[serde(default)]
    pub retries: u32,
}

impl Default for ServiceEndpoint {
    fn default() -> Self {
        Self {
            url: "http://localhost".to_string(),
            timeout_ms: 5000,
            retries: 3,
        }
    }
}
