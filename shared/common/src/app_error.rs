use thiserror::Error;

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
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::InvalidInput(_) => 400,
            AppError::NotFound(_) => 404,
            AppError::ConfigError(_) => 500,
            AppError::InternalError(_) => 500,
            AppError::BackendError(_) => 502,
            AppError::GrpcError(status) => match status.code() {
                tonic::Code::InvalidArgument => 400,
                tonic::Code::NotFound => 404,
                tonic::Code::Unauthenticated => 401,
                tonic::Code::PermissionDenied => 403,
                _ => 500,
            },
            AppError::IoError(_) => 500,
            AppError::Other(_) => 500,
        }
    }
}