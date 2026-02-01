use axum::routing::get;
use axum::Router;
use common::ServiceConfigLogic;
use gateway::{hello_handler, root_handler, GatewayConfig, GatewayError, GreeterServiceClient};
use std::net::SocketAddr;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), GatewayError> {
    // 1. Load the "Top Level" config only
    let config = GatewayConfig::load_auto()?;

    // 2. Access the shared logic through the nested 'base' field
    config.base.init_logging()?;

    info!(
        host = %config.server.host,
        port = %config.server.port,
        "Starting gateway"
    );

    let greeter = GreeterServiceClient::new(
        config.services.greeter.url,
        Some(Duration::from_millis(config.services.greeter.timeout_ms)),
    )
    .await?;

    // Build router
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/hello/{name}", get(hello_handler))
        .with_state(greeter);

    // Create socket address
    let addr = SocketAddr::from((
        config.server.host.parse::<std::net::IpAddr>()?,
        config.server.port,
    ));

    info!(%addr, "Gateway listening");

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// TODO macros for From implementation on Gateway Error - prettify error chain output print
// Error: Common(TransportError(tonic::transport::Error(Transport, ConnectError(ConnectError("tcp connect error", 127.0.0.1:50051, Os { code: 111, kind: ConnectionRefused, message: "Connection refused" })))))
// TODO mandatory secret
// #[derive(Deserialize, Validate)]
// pub struct DatabaseConfig {
//     #[validate(required(code = "missing_secret"))]
//     pub password: Option<String>,
// }
// #[validate(required)] is typically used on Option<T> types to ensure they aren't None.
// (code = "missing_secret") Overrides the default code and let us have a ley in the validation to derive the proper error message
// To make certain secrets mandatory it is possible to use the #[validate(required)] attribute
// to the sensitive fields in your Rust structs. If the secret isn't found in YAML, Env, or the Secret Dir,
// the config.validate() call at the end of the trait will catch it and prevent the service from running without a password.
