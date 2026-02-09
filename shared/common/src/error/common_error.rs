use macros::ExposeStructure;
use thiserror::Error;
use tonic::codegen::http::StatusCode;

/// Application-wide error type
#[derive(Error, Debug, ExposeStructure)]
pub enum CommonError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[bridge]
    #[error("Address parse error: {0}")]
    AddrParseError(#[from] std::net::AddrParseError),

    #[bridge]
    #[error("Transport/Connection error: {0}")]
    TransportError(#[from] tonic::transport::Error),

    #[bridge]
    #[error("gRPC status error: {0}")]
    GrpcError(#[from] tonic::Status),

    #[bridge]
    #[error("Invalid URI: {0}")]
    UriError(#[from] tonic::codegen::http::uri::InvalidUri),

    #[bridge]
    #[error("Reflection service error: {0}")]
    ReflectionError(#[from] tonic_reflection::server::Error),

    #[bridge]
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Missing mandatory secret: {0}")]
    MissingSecret(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl CommonError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            CommonError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            CommonError::NotFound(_) => StatusCode::NOT_FOUND,
            CommonError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CommonError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CommonError::TransportError(_) => StatusCode::BAD_GATEWAY,
            CommonError::GrpcError(status) => match status.code() {
                tonic::Code::InvalidArgument => StatusCode::BAD_REQUEST,
                tonic::Code::NotFound => StatusCode::NOT_FOUND,
                tonic::Code::Unauthenticated => StatusCode::UNAUTHORIZED,
                tonic::Code::PermissionDenied => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            CommonError::ReflectionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Helper for testing: checks if error matches a specific status code
    pub fn has_status(&self, code: u16) -> bool {
        self.status_code().as_u16() == code
    }

    /// Returns true if the error message contains the given substring
    pub fn contains(&self, message: &str) -> bool {
        // Use the Display implementation provided by thiserror
        format!("{}", self).to_string().contains(message)
    }
}
