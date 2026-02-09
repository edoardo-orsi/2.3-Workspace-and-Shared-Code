use tonic::transport::Server;
use tracing::info;
use common::{shutdown_signal, CommonError, ServiceConfigLogic};
use greeter_service::{GreeterConfig, GreeterService};
use proto_definitions::greeter_v1::DESCRIPTOR_SET;

#[tokio::main]
async fn main() -> Result<(), CommonError> {
    let config = GreeterConfig::load_auto()?;

    config.base.init_logging()?;

    info!(
        host = %config.server.host,
        port = %config.server.port,
        "Starting gateway"
    );

    let addr = format!("{}:{}", config.server.host, config.server.port).parse()?;

    // Create greeter service
    let greeter = GreeterService::server();

    // Create the reflection service
    let greeter_reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(DESCRIPTOR_SET)
        .build_v1()?;

    info!(%addr, "gRPC server listening");

    // Start server with graceful shutdown
    Server::builder()
        .add_service(greeter)
        .add_service(greeter_reflection_service)
        .serve_with_shutdown(addr, shutdown_signal())
        .await?;

    info!("Server shut down gracefully");

    Ok(())
}