use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};
use tracing::{debug, info, instrument};
use proto_definitions::greeter_v1::greeter_server::{Greeter, GreeterServer};
use proto_definitions::greeter_v1::{CustomHelloRequest, HelloRequest, HelloResponse};

#[derive(Debug, Default)]
pub struct GreeterService;

impl GreeterService {
    /// Create a new greeter service
    pub fn new() -> Self {
        Self
    }

    /// Create the gRPC server
    pub fn server() -> GreeterServer<Self> {
        GreeterServer::new(Self::new())
    }

    /// Get current Unix timestamp
    fn get_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64
    }
}

#[tonic::async_trait]
impl Greeter for GreeterService {
    /// Say hello to someone
    #[instrument(skip(self), fields(name = %request.get_ref().name))]
    async fn say_hello(&self, request: Request<HelloRequest>) -> Result<Response<HelloResponse>, Status> {
        let name = &request.get_ref().name;

        info!("Received SayHello request");

        // Validate input
        if name.is_empty() {
            debug!("Rejecting empty name");
            return Err(Status::invalid_argument("Name cannot be empty"));
        }

        if name.len() > 100 {
            debug!(name_length = name.len(), "Name too long");
            return Err(Status::invalid_argument("Name too long (max 100 characters)"));
        }

        // Create response
        let message = format!("Hello, {}!", name);
        let timestamp = Self::get_timestamp();

        debug!(message = %message, timestamp, "Sending response");

        Ok(Response::new(HelloResponse {
            message,
            timestamp,
        }))
    }

    /// Say hello with custom greeting
    #[instrument(
        skip(self),
        fields(
            name = %request.get_ref().name,
            greeting = %request.get_ref().greeting
        )
    )]
    async fn say_hello_custom(&self, request: Request<CustomHelloRequest>) -> Result<Response<HelloResponse>, Status> {
        let req = request.get_ref();
        let name = &req.name;
        let greeting = &req.greeting;

        info!("Received SayHelloCustom request");

        // Validate input
        if name.is_empty() {
            return Err(Status::invalid_argument("Name cannot be empty"));
        }

        if greeting.is_empty() {
            return Err(Status::invalid_argument("Greeting cannot be empty"));
        }

        // Create response with custom greeting
        let message = format!("{}, {}!", greeting, name);
        let timestamp = Self::get_timestamp();

        debug!(message = %message, "Sending custom response");

        Ok(Response::new(HelloResponse {
            message,
            timestamp,
        }))
    }
}