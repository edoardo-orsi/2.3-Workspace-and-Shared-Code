use thiserror::Error;
use tonic::codegen::http::StatusCode;

/// Application-wide error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("gRPC error: {0}")]
    GrpcError(#[from] tonic::Status),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BackendError(_) => StatusCode::BAD_GATEWAY,
            AppError::GrpcError(status) => match status.code() {
                tonic::Code::InvalidArgument => StatusCode::BAD_REQUEST,
                tonic::Code::NotFound => StatusCode::NOT_FOUND,
                tonic::Code::Unauthenticated => StatusCode::UNAUTHORIZED,
                tonic::Code::PermissionDenied => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
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