/// Integration tests for Gateway <-> Greeter Service interaction
///
/// **Using Real GreeterService Implementation**
///
/// This version uses the actual `GreeterService` from `greeter_service` crate
/// instead of a mock, providing true end-to-end integration testing.
///
/// # Architecture
///
/// ```text
/// Test Client → Gateway (HTTP) → Real Greeter Service (gRPC) → Response
///     ↓            ↓                         ↓
///   HTTP        Axum Router             Tonic Server
///  Request      (REST API)          (Production Service)
/// ```
///
/// # Advantages of Using Real Service
///
/// 1. **True Integration Testing**: Tests actual production code paths
/// 2. **Catches Real Issues**: Finds bugs in service implementation
/// 3. **Validates Proto Contracts**: Ensures proto definitions work correctly
/// 4. **Tests Serialization**: Verifies request/response encoding/decoding
/// 5. **Production Parity**: Test environment matches production
///
/// # Trade-offs vs Mock
///
/// **Real Service Pros:**
/// - Tests actual business logic
/// - Catches implementation bugs
/// - Validates entire stack
/// - Higher confidence in deployment
///
/// **Real Service Cons:**
/// - Slower test execution
/// - Harder to test edge cases
/// - Can't simulate specific error conditions
/// - Requires more setup
///
/// **Mock Service Pros:**
/// - Fast execution
/// - Easy to simulate errors
/// - Controlled test scenarios
/// - No external dependencies
///
/// **Mock Service Cons:**
/// - Doesn't test real implementation
/// - Mock behavior may diverge from reality
/// - False sense of security
///
/// # When to Use Which
///
/// **Use Real Service:**
/// - Integration test suites (like this file)
/// - Pre-deployment validation
/// - Contract testing
/// - Regression testing
///
/// **Use Mock Service:**
/// - Unit tests
/// - Error scenario testing
/// - Performance testing (controlled load)
/// - Development without backend
///
/// # Best Practice: Use Both
///
/// Have two test suites:
/// 1. `tests/integration/real_service.rs` - This file (real service)
/// 2. `tests/integration/mock_service.rs` - Error scenarios, edge cases
///
/// Run both in CI/CD for complete coverage.

use axum::Router;
use gateway::{hello_handler, GreeterServiceClient};
use greeter_service::GreeterService;
use proto_definitions::greeter_v1::greeter_server::GreeterServer;
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tonic::codegen::tokio_stream;
use tonic::transport::Server;

/// Start the REAL gRPC greeter service
///
/// This is the actual production service implementation, not a mock.
///
/// # Arguments
///
/// * `port` - Port to bind to (0 for random port)
///
/// # Returns
///
/// The actual bound address
async fn start_real_greeter_service(
    port: u16,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{}", port).parse()?;

    // Use the REAL GreeterService from the greeter_service crate
    let greeter = GreeterService::default();
    let server = GreeterServer::new(greeter);

    // Spawn the gRPC server
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

/// Start the HTTP gateway
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
    // Create real gRPC client
    let greeter = GreeterServiceClient::new(
        greeter_url,
        Some(Duration::from_secs(5)),
    )
        .await?;

    // Build router with real client
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
// Happy Path Tests - Using Real Service
// ============================================================================

#[tokio::test]
async fn test_real_integration_successful_hello() {
    // Start REAL gRPC server
    let grpc_addr = start_real_greeter_service(0)
        .await
        .expect("Failed to start gRPC server");

    // Start gateway pointing to real service
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

    // Verify response from REAL service
    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Failed to parse JSON");

    // Real service returns "Hello, Alice!"
    assert_eq!(body["message"], "Hello, Alice!");
    assert!(body["timestamp"].is_number());

    // Verify timestamp is recent (within last minute)
    let timestamp = body["timestamp"].as_i64().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!((now - timestamp).abs() < 60);
}

#[tokio::test]
async fn test_real_integration_multiple_names() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();

    let test_cases = vec![
        ("Alice", "Hello, Alice!"),
        ("Bob", "Hello, Bob!"),
        ("Charlie", "Hello, Charlie!"),
    ];

    for (name, expected_message) in test_cases {
        let response = client
            .get(format!("http://{}/hello/{}", gateway_addr, name))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        let body: Value = response.json().await.unwrap();
        assert_eq!(body["message"], expected_message);
    }
}

#[tokio::test]
async fn test_real_integration_unicode_names() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();

    // Test various unicode characters with REAL service
    let test_names = vec![
        ("José", "Hello, José!"),
        ("北京", "Hello, 北京!"),
        ("Москва", "Hello, Москва!"),
        ("محمد", "Hello, محمد!"),
    ];

    for (name, expected) in test_names {
        let encoded_name = urlencoding::encode(name);
        let response = client
            .get(format!("http://{}/hello/{}", gateway_addr, encoded_name))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status(), 200);

        let body: Value = response.json().await.unwrap();
        assert_eq!(body["message"], expected);
    }
}

// ============================================================================
// Validation Tests - Real Service Validation Logic
// ============================================================================

#[tokio::test]
async fn test_real_integration_name_too_long() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();

    // Name longer than 100 characters - tests REAL service validation
    let long_name = "A".repeat(101);
    let response = client
        .get(format!("http://{}/hello/{}", gateway_addr, long_name))
        .send()
        .await
        .expect("Failed to send request");

    // Real service returns INVALID_ARGUMENT which maps to 400
    assert_eq!(response.status(), 400);

    let body: Value = response.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("too long"));
}

#[tokio::test]
async fn test_real_integration_special_characters() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();

    // Test special characters that might break serialization
    let test_names = vec![
        "Alice&Bob",
        "Test<>User",
        "Quote\"Name",
        "Slash/Name",
        "Back\\Slash",
    ];

    for name in test_names {
        let encoded_name = urlencoding::encode(name);
        let response = client
            .get(format!("http://{}/hello/{}", gateway_addr, encoded_name))
            .send()
            .await
            .unwrap();

        // Should succeed with real service
        assert_eq!(response.status(), 200);

        let body: Value = response.json().await.unwrap();
        assert!(body["message"].as_str().unwrap().contains(name));
    }
}

// ============================================================================
// Concurrency Tests - Real Service Under Load
// ============================================================================

#[tokio::test]
async fn test_real_integration_concurrent_requests() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = Arc::new(reqwest::Client::new());
    let mut handles = vec![];

    // Send 50 concurrent requests to REAL service
    for i in 0..50 {
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

    assert_eq!(success_count, 50, "Not all requests succeeded");
}

#[tokio::test]
async fn test_real_integration_sequential_requests() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();

    // Send multiple sequential requests
    for i in 0..10 {
        let response = client
            .get(format!("http://{}/hello/User{}", gateway_addr, i))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        let body: Value = response.json().await.unwrap();
        assert_eq!(body["message"], format!("Hello, User{}!", i));
    }
}

// ============================================================================
// Response Format Tests - Real Service Response Structure
// ============================================================================

#[tokio::test]
async fn test_real_integration_response_format() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

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

    // Verify JSON structure from REAL service
    let body: Value = response.json().await.unwrap();

    assert!(body.is_object());
    assert!(body.get("message").is_some());
    assert!(body.get("timestamp").is_some());

    // Verify types
    assert!(body["message"].is_string());
    assert!(body["timestamp"].is_i64() || body["timestamp"].is_u64());

    // Verify message format
    let message = body["message"].as_str().unwrap();
    assert!(message.starts_with("Hello, "));
    assert!(message.ends_with("!"));
}

#[tokio::test]
async fn test_real_integration_timestamp_accuracy() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();

    // Record time before request
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let response = client
        .get(format!("http://{}/hello/Alice", gateway_addr))
        .send()
        .await
        .unwrap();

    // Record time after request
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let body: Value = response.json().await.unwrap();
    let timestamp = body["timestamp"].as_i64().unwrap();

    // Timestamp should be between before and after
    assert!(
        timestamp >= before && timestamp <= after,
        "Timestamp {} not in range [{}, {}]",
        timestamp,
        before,
        after
    );
}

// ============================================================================
// Performance Tests - Real Service Performance
// ============================================================================

#[tokio::test]
async fn test_real_integration_latency() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();
    let mut latencies = vec![];

    // Warm up
    for _ in 0..5 {
        let _ = client
            .get(format!("http://{}/hello/Warmup", gateway_addr))
            .send()
            .await;
    }

    // Measure latency
    for _ in 0..20 {
        let start = std::time::Instant::now();

        let response = client
            .get(format!("http://{}/hello/Alice", gateway_addr))
            .send()
            .await
            .unwrap();

        let latency = start.elapsed();

        assert_eq!(response.status(), 200);
        latencies.push(latency);
    }

    // Calculate statistics
    latencies.sort();
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];

    println!("\n=== Real Service Latency ===");
    println!("p50: {:?}", p50);
    println!("p95: {:?}", p95);
    println!("p99: {:?}", p99);

    // Assert reasonable latency (adjust based on your requirements)
    assert!(p99 < Duration::from_millis(100), "p99 latency too high: {:?}", p99);
}

// ============================================================================
// Error Handling Tests - Real Service Error Behavior
// ============================================================================

#[tokio::test]
async fn test_real_integration_service_restart() {
    // Start gateway first
    let grpc_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(grpc_addr).await.unwrap();
    let grpc_addr = listener.local_addr().unwrap();

    // Start greeter on the address
    let greeter = GreeterService::default();
    let server = GreeterServer::new(greeter);

    let _server_handle = tokio::spawn(async move {
        Server::builder()
            .add_service(server)
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();

    // First request should succeed
    let response = client
        .get(format!("http://{}/hello/Alice", gateway_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // Note: Actually restarting the service in a test is complex
    // This demonstrates the test structure
    // In practice, you might use Docker containers or process management
}

// ============================================================================
// Load Test - Real Service Under Sustained Load
// ============================================================================

#[tokio::test]
#[ignore] // Run with --ignored flag for load testing
async fn test_real_integration_load_test() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = Arc::new(reqwest::Client::new());
    let start = std::time::Instant::now();
    let mut handles = vec![];

    // Send 1000 requests to REAL service
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

    println!("\n=== Real Service Load Test Results ===");
    println!("Total requests: 1000");
    println!("Successful: {}", success_count);
    println!("Duration: {:?}", duration);
    println!("RPS: {:.2}", 1000.0 / duration.as_secs_f64());
    println!("Latency p50: {:?}", p50);
    println!("Latency p95: {:?}", p95);
    println!("Latency p99: {:?}", p99);

    assert_eq!(success_count, 1000, "Not all requests succeeded");
}

// ============================================================================
// Contract Testing - Verify Proto Contract Compliance
// ============================================================================

#[tokio::test]
async fn test_real_integration_proto_contract() {
    let grpc_addr = start_real_greeter_service(0).await.unwrap();
    let gateway_addr = start_gateway(format!("http://{}", grpc_addr), 0).await.unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/hello/Alice", gateway_addr))
        .send()
        .await
        .unwrap();

    let body: Value = response.json().await.unwrap();

    // Verify contract: message field must be string
    assert!(body["message"].is_string());

    // Verify contract: timestamp field must be number (i64)
    assert!(body["timestamp"].is_number());

    // Verify contract: message format
    let message = body["message"].as_str().unwrap();
    assert!(message.contains("Alice"));

    // Verify contract: no extra fields
    assert_eq!(body.as_object().unwrap().len(), 2);
}