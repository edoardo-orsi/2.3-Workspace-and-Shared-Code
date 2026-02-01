use crate::services::greeter::greeter_client::GreeterServiceClient;
use crate::GatewayError;
use axum::extract::{Path, State};
use axum::{debug_handler, Json};
use proto_definitions::greeter_v1::HelloResponse;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, instrument};

#[debug_handler]
#[instrument(skip(greeter))]
// The skip argument in the instrument macro must match the variable name
// of the function argument you want to ignore
pub async fn hello_handler(
    Path(name): Path<String>,
    State(greeter): State<GreeterServiceClient>,
) -> Result<Json<HelloResponse>, GatewayError> {
    info!(name = %name, "HTTP hello request received");

    let mut hello_response = greeter.say_hello(name).await?;

    hello_response.timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    info!(message = %hello_response.message, "Successfully processed request");

    Ok(Json(hello_response))
}
