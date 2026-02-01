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
