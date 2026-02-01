use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use common::{CommonError, ErrorLogic};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error(transparent)]
    Common(#[from] CommonError),

    #[error("Gateway specific logic error: {0}")]
    GatewaySpecific(String),
}

common::propagate_commonerror!(GatewayError, Common);

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
