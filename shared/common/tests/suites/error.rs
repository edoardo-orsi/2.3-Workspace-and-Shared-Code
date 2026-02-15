#[cfg(test)]
mod tests {
    use tonic::codegen::http::StatusCode;
    use common::CommonError;

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