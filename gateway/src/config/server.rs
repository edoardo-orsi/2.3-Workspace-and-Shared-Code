use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Validate)]
pub struct ServerConfig {
    #[validate(length(min = 1, message = "Host name cannot be empty"))]
    pub host: String,

    #[validate(range(
        min = 1,
        max = 65535,
        message = "Port cannot be less than 1 or higher than 65.535"
    ))]
    pub port: u16,
}
