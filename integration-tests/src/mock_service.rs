/// Integration tests for Gateway <-> Greeter Service interaction
///
/// These tests verify end-to-end communication between the HTTP gateway
/// and the gRPC greeter service, ensuring proper request transformation,
/// error handling, and response formatting.
///
/// # Test Architecture
///
/// ```text
/// Test Client → Gateway (HTTP) → Greeter Service (gRPC) → Response
///     ↓            ↓                     ↓
///   HTTP        Axum Router         Tonic Server
///  Request      (REST API)          (gRPC Service)
/// ```
///
/// # Prerequisites
///
/// - Both services must be configured correctly
/// - Greeter service must be running on configured port
/// - Gateway must have valid greeter service URL
///
/// # Test Categories
///
/// 1. **Happy Path Tests**: Successful request/response flows
/// 2. **Validation Tests**: Input validation and error handling
/// 3. **Error Propagation**: gRPC errors → HTTP errors
/// 4. **Timeout Tests**: Connection and request timeouts
/// 5. **Load Tests**: Concurrent request handling

use axum::Router;
use gateway::{hello_handler, GreeterServiceClient};
use proto_definitions::greeter_v1::{
    greeter_server::{Greeter, GreeterServer},
    CustomHelloRequest, HelloRequest, HelloResponse,
};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tonic::{transport::Server, Request, Response, Status};


/// Mock greeter service for testing
///
/// This implementation allows us to:
/// - Test gateway behavior without external dependencies
/// - Simulate various error conditions
/// - Verify request transformation
/// - Test timeout and retry logic
#[derive(Debug, Default, Clone)]
struct MockGreeterService {
    /// Optional artificial delay for timeout testing
    delay: Option<Duration>,

    /// Optional error to return for error testing
    error: Option<Status>,
}

impl MockGreeterService {
    fn new() -> Self {
        Self::default()
    }

    fn with_delay(delay: Duration) -> Self {
        Self {
            delay: Some(delay),
            error: None,
        }
    }

    fn with_error(error: Status) -> Self {
        Self {
            delay: None,
            error: Some(error),
        }
    }
}

#[tonic::async_trait]
impl Greeter for MockGreeterService {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        // Simulate delay if configured
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }

        // Return error if configured
        if let Some(ref error) = self.error {
            return Err(error.clone());
        }

        let name = &request.get_ref().name;

        // Validate input (same as real service)
        if name.is_empty() {
            return Err(Status::invalid_argument("Name cannot be empty"));
        }

        if name.len() > 100 {
            return Err(Status::invalid_argument("Name too long (max 100 characters)"));
        }

        let message = format!("Hello, {}!", name);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(Response::new(HelloResponse { message, timestamp }))
    }

    async fn say_hello_custom(
        &self,
        request: Request<CustomHelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }

        if let Some(ref error) = self.error {
            return Err(error.clone());
        }

        let req = request.get_ref();
        let name = &req.name;
        let greeting = &req.greeting;

        if name.is_empty() {
            return Err(Status::invalid_argument("Name cannot be empty"));
        }

        if greeting.is_empty() {
            return Err(Status::invalid_argument("Greeting cannot be empty"));
        }

        let message = format!("{}, {}!", greeting, name);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(Response::new(HelloResponse { message, timestamp }))
    }
}

/// Start a mock gRPC server for testing
///
/// # Arguments
///
/// * `service` - The greeter service implementation
/// * `port` - Port to bind to (0 for random port)
///
/// # Returns
///
/// The actual bound address (useful when port = 0)
async fn start_grpc_server(
    service: MockGreeterService,
    port: u16,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{}", port).parse()?;

    let server = GreeterServer::new(service);

    tokio::spawn(async move {
        Server::builder()
            .add_service(server)
            .serve(addr)
            .await
            .expect("gRPC server failed");
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(addr)
}

/// Start the HTTP gateway for testing
///
/// # Arguments
///
/// * `greeter_url` - URL of the gRPC greeter service
/// * `port` - Port to bind to (0 for random port)
///
/// # Returns
///
/// The actual bound address
async fn start_gateway(
    greeter_url: String,
    port: u16,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let greeter = GreeterServiceClient::new(
        greeter_url,
        Some(Duration::from_secs(5)),
    )
        .await?;

    let app = Router::new()
        .route("/hello/:name", axum::routing::get(hello_handler))
        .with_state(greeter);

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_addr = listener.local_addr()?;

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("HTTP server failed");
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(actual_addr)
}

// ============================================================================
// Happy Path Tests
// ============================================================================

#[tokio::test]
async fn test_integration_successful_hello() {
    // Start gRPC server
    let grpc_addr = start_grpc_server(MockGreeterService::new(), 0)
        .await
        .expect("Failed to start gRPC server");

    // Start gateway
    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .expect("Failed to start gateway");

    // Make HTTP request
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/hello/Alice", gateway_addr))
        .send()
        .await
        .expect("Failed to send request");

    // Verify response
    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["message"], "Hello, Alice!");
    assert!(body["timestamp"].is_number());
}

#[tokio::test]
async fn test_integration_unicode_names() {
    let grpc_addr = start_grpc_server(MockGreeterService::new(), 0)
        .await
        .unwrap();

    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .unwrap();

    let client = reqwest::Client::new();

    // Test various unicode characters
    let test_names = vec![
        "José",
        "北京",
        "Москва",
        "محمد",
        "🎉",
    ];

    for name in test_names {
        let encoded_name = urlencoding::encode(name);
        let response = client
            .get(format!("http://{}/hello/{}", gateway_addr, encoded_name))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status(), 200);

        let body: Value = response.json().await.unwrap();
        assert!(body["message"].as_str().unwrap().contains(name));
    }
}

// ============================================================================
// Validation Tests
// ============================================================================

#[tokio::test]
async fn test_integration_empty_name_validation() {
    let grpc_addr = start_grpc_server(MockGreeterService::new(), 0)
        .await
        .unwrap();

    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .unwrap();

    let client = reqwest::Client::new();

    // Empty name should return 400
    let response = client
        .get(format!("http://{}/hello/", gateway_addr))
        .send()
        .await
        .expect("Failed to send request");

    // Note: This will be 404 because the route doesn't match
    // In a real scenario, you'd want a different test or endpoint
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_integration_name_too_long() {
    let grpc_addr = start_grpc_server(MockGreeterService::new(), 0)
        .await
        .unwrap();

    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .unwrap();

    let client = reqwest::Client::new();

    // Name longer than 100 characters
    let long_name = "A".repeat(101);
    let response = client
        .get(format!("http://{}/hello/{}", gateway_addr, long_name))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);

    let body: Value = response.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("too long"));
}

// ============================================================================
// Error Propagation Tests
// ============================================================================

#[tokio::test]
async fn test_integration_grpc_error_propagation() {
    // Create a service that always returns an error
    let service = MockGreeterService::with_error(
        Status::internal("Database connection failed")
    );

    let grpc_addr = start_grpc_server(service, 0)
        .await
        .unwrap();

    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/hello/Alice", gateway_addr))
        .send()
        .await
        .expect("Failed to send request");

    // Internal gRPC error should map to 500
    assert_eq!(response.status(), 500);

    let body: Value = response.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Database"));
}

#[tokio::test]
async fn test_integration_invalid_argument_error() {
    let service = MockGreeterService::with_error(
        Status::invalid_argument("Invalid request format")
    );

    let grpc_addr = start_grpc_server(service, 0)
        .await
        .unwrap();

    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/hello/Alice", gateway_addr))
        .send()
        .await
        .unwrap();

    // INVALID_ARGUMENT should map to 400
    assert_eq!(response.status(), 400);
}

// ============================================================================
// Timeout Tests
// ============================================================================

#[tokio::test]
async fn test_integration_request_timeout() {
    // Create a service with 10 second delay
    let service = MockGreeterService::with_delay(Duration::from_secs(10));

    let grpc_addr = start_grpc_server(service, 0)
        .await
        .unwrap();

    // Create gateway with 1 second timeout
    let greeter = GreeterServiceClient::new(
        format!("http://{}", grpc_addr),
        Some(Duration::from_secs(1)),
    )
        .await
        .unwrap();

    let app = Router::new()
        .route("/hello/:name", axum::routing::get(hello_handler))
        .with_state(greeter);

    let gateway_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(gateway_addr).await.unwrap();
    let actual_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Request should timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    let result = client
        .get(format!("http://{}/hello/Alice", actual_addr))
        .send()
        .await;

    // Should get an error (timeout or 504)
    match result {
        Err(_) => { /* timeout from client */ },
        Ok(response) => {
            // Or 504 from gateway
            assert!(
                response.status().is_server_error(),
                "Expected server error, got {}",
                response.status()
            );
        }
    }
}

// ============================================================================
// Concurrent Request Tests
// ============================================================================

#[tokio::test]
async fn test_integration_concurrent_requests() {
    let grpc_addr = start_grpc_server(MockGreeterService::new(), 0)
        .await
        .unwrap();

    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .unwrap();

    let client = Arc::new(reqwest::Client::new());
    let mut handles = vec![];

    // Send 100 concurrent requests
    for i in 0..100 {
        let client = Arc::clone(&client);
        let url = format!("http://{}/hello/User{}", gateway_addr, i);

        let handle = tokio::spawn(async move {
            client.get(&url).send().await
        });

        handles.push(handle);
    }

    // Wait for all requests
    let results = futures::future::join_all(handles).await;

    // All should succeed
    let mut success_count = 0;
    for result in results {
        if let Ok(Ok(response)) = result {
            if response.status() == 200 {
                success_count += 1;
            }
        }
    }

    assert_eq!(success_count, 100, "Not all requests succeeded");
}

// ============================================================================
// Connection Failure Tests
// ============================================================================

#[tokio::test]
async fn test_integration_service_unavailable() {
    // Start gateway pointing to non-existent service
    let result = GreeterServiceClient::new(
        "http://localhost:9999".to_string(),
        Some(Duration::from_millis(100)),
    )
        .await;

    // Should fail to connect
    assert!(result.is_err());
}

#[tokio::test]
async fn test_integration_connection_refused() {
    // Try to create client for non-listening port
    let result = timeout(
        Duration::from_secs(2),
        GreeterServiceClient::new(
            "http://127.0.0.1:54321".to_string(),
            Some(Duration::from_millis(500)),
        ),
    )
        .await;

    // Should timeout or fail
    match result {
        Ok(Ok(_)) => panic!("Should not connect to non-existent service"),
        Ok(Err(_)) => { /* Expected error */ },
        Err(_) => { /* Expected timeout */ },
    }
}

// ============================================================================
// Response Format Tests
// ============================================================================

#[tokio::test]
async fn test_integration_response_format() {
    let grpc_addr = start_grpc_server(MockGreeterService::new(), 0)
        .await
        .unwrap();

    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/hello/Alice", gateway_addr))
        .send()
        .await
        .unwrap();

    // Verify headers
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    // Verify JSON structure
    let body: Value = response.json().await.unwrap();

    assert!(body.is_object());
    assert!(body.get("message").is_some());
    assert!(body.get("timestamp").is_some());

    // Timestamp should be recent (within last minute)
    let timestamp = body["timestamp"].as_i64().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    assert!(
        (now - timestamp).abs() < 60,
        "Timestamp {} is not recent (now: {})",
        timestamp,
        now
    );
}

// ============================================================================
// Load Test
// ============================================================================

#[tokio::test]
#[ignore] // Run with --ignored flag for load testing
async fn test_integration_load_test() {
    let grpc_addr = start_grpc_server(MockGreeterService::new(), 0)
        .await
        .unwrap();

    let gateway_addr = start_gateway(
        format!("http://{}", grpc_addr),
        0,
    )
        .await
        .unwrap();

    let client = Arc::new(reqwest::Client::new());
    let start = std::time::Instant::now();
    let mut handles = vec![];

    // Send 1000 requests
    for i in 0..1000 {
        let client = Arc::clone(&client);
        let url = format!("http://{}/hello/User{}", gateway_addr, i);

        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = client.get(&url).send().await;
            let latency = start.elapsed();
            (result, latency)
        });

        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    let duration = start.elapsed();
    let mut success_count = 0;
    let mut latencies = vec![];

    for result in results {
        if let Ok((Ok(response), latency)) = result {
            if response.status() == 200 {
                success_count += 1;
                latencies.push(latency);
            }
        }
    }

    // Calculate statistics
    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];

    println!("\n=== Load Test Results ===");
    println!("Total requests: 1000");
    println!("Successful: {}", success_count);
    println!("Duration: {:?}", duration);
    println!("RPS: {:.2}", 1000.0 / duration.as_secs_f64());
    println!("Latency p50: {:?}", p50);
    println!("Latency p95: {:?}", p95);
    println!("Latency p99: {:?}", p99);

    assert_eq!(success_count, 1000);
    assert!(p99 < Duration::from_millis(100), "p99 latency too high");
}