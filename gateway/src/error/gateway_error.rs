use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use common::{CommonError, ErrorLogic};
use serde_json::json;
use std::error::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error(transparent)]
    Common(#[from] CommonError),

    #[error("Gateway specific logic error: {0}")]
    GatewaySpecific(String),
}

impl From<std::io::Error> for GatewayError {
    fn from(err: std::io::Error) -> Self {
        GatewayError::Common(CommonError::IoError(err))
    }
}

impl From<tonic::Status> for GatewayError {
    fn from(err: tonic::Status) -> Self {
        GatewayError::Common(CommonError::GrpcError(err))
    }
}

impl From<tonic::transport::Error> for GatewayError {
    fn from(err: tonic::transport::Error) -> Self {
        GatewayError::Common(CommonError::TransportError(err))
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = match &self {
            GatewayError::Common(inner) => inner.status_code(),
            GatewayError::GatewaySpecific(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
