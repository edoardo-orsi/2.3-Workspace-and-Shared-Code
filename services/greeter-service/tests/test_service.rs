use greeter_service::GreeterService;
use proto_definitions::greeter_v1::greeter_server::Greeter;
use proto_definitions::greeter_v1::{CustomHelloRequest, HelloRequest};
use tonic::{Code, Request};

/// Test helper: Create a service instance
fn service() -> GreeterService {
    GreeterService::new()
}

#[tokio::test]
async fn test_say_hello_success() {
    let svc = service();
    let req = Request::new(HelloRequest {
        name: "Alice".to_string(),
    });

    let response = svc.say_hello(req).await.unwrap();
    let hello = response.into_inner();

    assert_eq!(hello.message, "Hello, Alice!");
    assert!(hello.timestamp > 0);
}

#[tokio::test]
async fn test_say_hello_empty_name() {
    let svc = service();
    let req = Request::new(HelloRequest {
        name: "".to_string(),
    });

    let result = svc.say_hello(req).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
    assert!(status.message().contains("empty"));
}

#[tokio::test]
async fn test_say_hello_name_too_long() {
    let svc = service();
    let req = Request::new(HelloRequest {
        name: "A".repeat(101),
    });

    let result = svc.say_hello(req).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
    assert!(status.message().contains("too long"));
}

#[tokio::test]
async fn test_say_hello_unicode() {
    let svc = service();
    let req = Request::new(HelloRequest {
        name: "世界".to_string(),
    });

    let response = svc.say_hello(req).await.unwrap();
    let hello = response.into_inner();

    assert_eq!(hello.message, "Hello, 世界!");
}

#[tokio::test]
async fn test_say_hello_custom_success() {
    let svc = service();
    let req = Request::new(CustomHelloRequest {
        name: "World".to_string(),
        greeting: "Howdy".to_string(),
    });

    let response = svc.say_hello_custom(req).await.unwrap();
    let hello = response.into_inner();

    assert_eq!(hello.message, "Howdy, World!");
    assert!(hello.timestamp > 0);
}

#[tokio::test]
async fn test_say_hello_custom_empty_greeting() {
    let svc = service();
    let req = Request::new(CustomHelloRequest {
        name: "Alice".to_string(),
        greeting: "".to_string(),
    });

    let result = svc.say_hello_custom(req).await;
    assert!(result.is_err());

    let status = result.unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
    assert!(status.message().contains("Greeting"));
}

#[tokio::test]
async fn test_timestamp_is_recent() {
    let svc = service();
    let req = Request::new(HelloRequest {
        name: "Test".to_string(),
    });

    let before = GreeterService::get_timestamp();
    let response = svc.say_hello(req).await.unwrap();
    let after = GreeterService::get_timestamp();

    let timestamp = response.into_inner().timestamp;

    assert!(timestamp >= before && timestamp <= after);
}
