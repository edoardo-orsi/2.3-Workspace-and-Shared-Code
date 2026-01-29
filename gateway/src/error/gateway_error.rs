use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use common::CommonError;
use serde_json::json;
use std::error::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error(transparent)]
    Common(#[from] CommonError),

    #[error("Invalid URI: {0}")]
    UriError(#[from] tonic::codegen::http::uri::InvalidUri),

    #[error(transparent)]
    TransportError(#[from] tonic::transport::Error),

    #[error(transparent)]
    GrpcStatus(#[from] tonic::Status),

    #[error("Internal gateway error: {0}")]
    Internal(String),
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = match &self {
            GatewayError::Common(inner) => inner.status_code(),
            GatewayError::UriError(_) => StatusCode::BAD_REQUEST,
            GatewayError::TransportError(_) => StatusCode::BAD_GATEWAY,
            GatewayError::GrpcStatus(s) => CommonError::GrpcError(s.clone()).status_code(),
            GatewayError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // This gathers all the "why" messages in the chain
        let full_details = self.report_chain();

        let body = Json(json!({
            "error": self.to_string(),
            "details": full_details,
            "code": status.as_u16()
        }));

        (status, body).into_response()
    }
}

impl GatewayError {
    /// Iterates through the source chain to build a full error report
    fn report_chain(&self) -> Vec<String> {
        let mut chain = Vec::new();
        let mut curr: Option<&dyn Error> = Some(self);

        while let Some(source) = curr {
            chain.push(source.to_string());
            curr = source.source();
        }
        chain
    }
}
