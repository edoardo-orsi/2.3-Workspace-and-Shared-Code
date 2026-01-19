use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Validate)]
pub struct ServiceConfig {
    #[validate(length(min = 1, message = "Service name cannot be empty"))]
    pub name: String,

    #[validate(length(min = 1, message = "Service version cannot be empty"))]
    pub version: String,
}
