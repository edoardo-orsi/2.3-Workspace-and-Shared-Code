use macros::ExposeStructure;
use thiserror::Error;
use tonic::codegen::http::StatusCode;

/// Application-wide error type that serves as the foundational error enum
/// for the entire microservices' architecture.
///
/// # Purpose
///
/// This enum centralizes error handling across all services, providing:
/// - Consistent error representation
/// - HTTP status code mapping for REST APIs
/// - gRPC status code mapping for gRPC services
/// - Error chain reporting for debugging
/// - Automatic conversion from common error types
///
/// # Error Bridging
///
/// The `#[bridge]` attribute enables automatic error conversion. When an error
/// type is marked with `#[bridge]`, the `ExposeStructure` derive macro generates
/// `From` implementations that allow seamless error propagation across service
/// boundaries.
///
/// # Examples
///
/// ```
/// use common::CommonError;
///
/// fn read_config() -> Result<String, CommonError> {
///     // IO errors automatically convert to CommonError
///     std::fs::read_to_string("config.yaml")?;
///     Ok("loaded".to_string())
/// }
///
/// fn validate_input(value: &str) -> Result<(), CommonError> {
///     if value.is_empty() {
///         return Err(CommonError::InvalidInput(
///             "Value cannot be empty".to_string()
///         ));
///     }
///     Ok(())
/// }
/// ```
///
/// # HTTP Status Code Mapping
///
/// Each error variant maps to an appropriate HTTP status code:
/// - `InvalidInput` → 400 Bad Request
/// - `NotFound` → 404 Not Found
/// - `ConfigError` → 500 Internal Server Error
/// - `TransportError` → 502 Bad Gateway
/// - `GrpcError` → Contextual (based on gRPC code)
///
/// # Service Integration
///
/// Services create their own error types that wrap `CommonError`:
///
/// ```ignore
/// #[derive(Error, Debug, ExposeStructure)]
/// pub enum GatewayError {
///     #[bridge]
///     #[error(transparent)]
///     Common(#[from] CommonError),
///
///     #[error("Gateway specific error: {0}")]
///     GatewaySpecific(String),
/// }
///
/// common::propagate_commonerror!(GatewayError, Common);
/// ```
#[derive(Error, Debug, ExposeStructure)]
pub enum CommonError {
    /// Configuration loading or validation error
    ///
    /// Returned when:
    /// - Config file is missing or malformed
    /// - Environment variable parsing fails
    /// - Validation constraints are violated
    /// - Secret files are inaccessible
    ///
    /// # HTTP Status: 500 Internal Server Error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Invalid input provided by client
    ///
    /// Returned when:
    /// - Request parameters are invalid
    /// - Required fields are missing
    /// - Data format is incorrect
    /// - Business rule validation fails
    ///
    /// # HTTP Status: 400 Bad Request
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Requested resource does not exist
    ///
    /// Returned when:
    /// - Entity not found in database
    /// - Service endpoint doesn't exist
    /// - File or resource is missing
    ///
    /// # HTTP Status: 404 Not Found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Unexpected internal server error
    ///
    /// Returned when:
    /// - Unrecoverable application state
    /// - Unexpected panic recovery
    /// - Resource exhaustion
    ///
    /// # HTTP Status: 500 Internal Server Error
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// Socket address parsing failed
    ///
    /// Automatically converted from `std::net::AddrParseError`.
    /// Occurs when binding to invalid host:port combinations.
    ///
    /// # HTTP Status: 500 Internal Server Error
    #[bridge]
    #[error("Address parse error: {0}")]
    AddrParseError(#[from] std::net::AddrParseError),

    /// gRPC transport or connection error
    ///
    /// Automatically converted from `tonic::transport::Error`.
    /// Occurs when:
    /// - Connection to backend service fails
    /// - TLS handshake fails
    /// - Network timeout occurs
    ///
    /// # HTTP Status: 502 Bad Gateway
    #[bridge]
    #[error("Transport/Connection error: {0}")]
    TransportError(#[from] tonic::transport::Error),

    /// gRPC status error from remote service
    ///
    /// Automatically converted from `tonic::Status`.
    /// Maps gRPC status codes to HTTP status codes:
    /// - `INVALID_ARGUMENT` → 400
    /// - `NOT_FOUND` → 404
    /// - `UNAUTHENTICATED` → 401
    /// - `PERMISSION_DENIED` → 403
    /// - Others → 500
    ///
    /// # HTTP Status: Contextual (see status_code method)
    #[bridge]
    #[error("gRPC status error: {0}")]
    GrpcError(#[from] tonic::Status),

    /// Invalid URI format
    ///
    /// Automatically converted from `http::uri::InvalidUri`.
    /// Occurs when constructing gRPC client connections with
    /// malformed URIs.
    ///
    /// # HTTP Status: 500 Internal Server Error
    #[bridge]
    #[error("Invalid URI: {0}")]
    UriError(#[from] tonic::codegen::http::uri::InvalidUri),

    /// gRPC reflection service error
    ///
    /// Automatically converted from `tonic_reflection::server::Error`.
    /// Occurs during reflection service setup or descriptor registration.
    ///
    /// # HTTP Status: 500 Internal Server Error
    #[bridge]
    #[error("Reflection service error: {0}")]
    ReflectionError(#[from] tonic_reflection::server::Error),

    /// File system I/O error
    ///
    /// Automatically converted from `std::io::Error`.
    /// Occurs when:
    /// - Reading/writing config files
    /// - Accessing secret files
    /// - Creating directories
    ///
    /// # HTTP Status: 500 Internal Server Error
    #[bridge]
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Required secret is missing
    ///
    /// Returned when:
    /// - Kubernetes secret not mounted
    /// - Docker secret file missing
    /// - Required password/key not found
    ///
    /// This is a critical error that should prevent service startup.
    ///
    /// # HTTP Status: 500 Internal Server Error
    #[error("Missing mandatory secret: {0}")]
    MissingSecret(String),

    /// Catch-all for other error types
    ///
    /// Use sparingly; prefer specific variants or bridging.
    ///
    /// # HTTP Status: 500 Internal Server Error
    #[error("Other error: {0}")]
    Other(String),
}

impl CommonError {
    /// Maps the error to an HTTP status code
    ///
    /// This method is used by REST API handlers to determine the
    /// appropriate HTTP response status.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::CommonError;
    /// use tonic::codegen::http::StatusCode;
    ///
    /// let err = CommonError::InvalidInput("Bad data".to_string());
    /// assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    ///
    /// let err = CommonError::NotFound("User not found".to_string());
    /// assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    /// ```
    ///
    /// # gRPC Status Mapping
    ///
    /// For `GrpcError` variants, the gRPC status code is mapped:
    /// - `INVALID_ARGUMENT` → 400 Bad Request
    /// - `NOT_FOUND` → 404 Not Found
    /// - `UNAUTHENTICATED` → 401 Unauthorized
    /// - `PERMISSION_DENIED` → 403 Forbidden
    /// - All others → 500 Internal Server Error
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
                tonic::Code::AlreadyExists => StatusCode::CONFLICT,
                tonic::Code::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
                tonic::Code::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
                tonic::Code::Aborted => StatusCode::CONFLICT,
                tonic::Code::OutOfRange => StatusCode::BAD_REQUEST,
                tonic::Code::Unimplemented => StatusCode::NOT_IMPLEMENTED,
                tonic::Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                tonic::Code::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            CommonError::ReflectionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CommonError::MissingSecret(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Helper for testing: checks if error matches a specific status code
    ///
    /// # Examples
    ///
    /// ```
    /// use common::CommonError;
    ///
    /// let err = CommonError::NotFound("User".to_string());
    /// assert!(err.has_status(404));
    /// assert!(!err.has_status(500));
    /// ```
    #[cfg(test)]
    pub fn has_status(&self, code: u16) -> bool {
        self.status_code().as_u16() == code
    }

    /// Returns true if the error message contains the given substring
    ///
    /// Useful for testing and error matching in handlers.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::CommonError;
    ///
    /// let err = CommonError::ConfigError("Missing field: database_url".to_string());
    /// assert!(err.contains("database_url"));
    /// assert!(!err.contains("port"));
    /// ```
    #[cfg(test)]
    pub fn contains(&self, message: &str) -> bool {
        // Use the Display implementation provided by thiserror
        format!("{}", self).to_string().contains(message)
    }

    /// Converts the error into a gRPC Status
    ///
    /// Used by gRPC service handlers to return appropriate status codes.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// async fn my_rpc(&self, request: Request<MyRequest>)
    ///     -> Result<Response<MyResponse>, Status>
    /// {
    ///     let result = do_something().map_err(|e| e.into_grpc_status())?;
    ///     Ok(Response::new(result))
    /// }
    /// ```
    pub fn into_grpc_status(self) -> tonic::Status {
        match self {
            CommonError::InvalidInput(msg) => {
                tonic::Status::invalid_argument(msg)
            }
            CommonError::NotFound(msg) => {
                tonic::Status::not_found(msg)
            }
            CommonError::GrpcError(status) => status,
            CommonError::MissingSecret(msg) => {
                tonic::Status::failed_precondition(format!("Missing secret: {}", msg))
            }
            other => tonic::Status::internal(other.to_string()),
        }
    }

    /// Creates a ConfigError from a validation error
    ///
    /// Helper for converting validator::ValidationErrors into CommonError.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// config.validate()
    ///     .map_err(CommonError::from_validation)?;
    /// ```
    pub fn from_validation(err: validator::ValidationErrors) -> Self {
        CommonError::ConfigError(format!("Validation failed: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_code_mapping() {
        assert_eq!(
            CommonError::InvalidInput("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            CommonError::NotFound("test".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            CommonError::ConfigError("test".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_grpc_status_mapping() {
        let invalid_arg = CommonError::GrpcError(
            tonic::Status::invalid_argument("bad input")
        );
        assert_eq!(invalid_arg.status_code(), StatusCode::BAD_REQUEST);

        let not_found = CommonError::GrpcError(
            tonic::Status::not_found("not found")
        );
        assert_eq!(not_found.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_contains() {
        let err = CommonError::ConfigError("Missing database_url".to_string());
        assert!(err.contains("database_url"));
        assert!(!err.contains("port"));
    }

    #[test]
    fn test_into_grpc_status() {
        let err = CommonError::InvalidInput("bad data".to_string());
        let status = err.into_grpc_status();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn test_error_display() {
        let err = CommonError::NotFound("User".to_string());
        assert_eq!(err.to_string(), "Not found: User");
    }
}