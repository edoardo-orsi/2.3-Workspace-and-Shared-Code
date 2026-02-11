# Gateway Service

## Overview

The Gateway Service acts as the HTTP/REST entry point to your microservices architecture. It translates HTTP requests into gRPC calls to backend services, providing a unified API interface for external clients.

## Architecture Role

```
Client (HTTP) → Gateway (HTTP → gRPC) → Backend Services (gRPC)
```

### Interactions with Other Projects

1. **Common Library** (`shared/common`)
    - Uses `BaseConfig` for configuration management
    - Uses `CommonError` for standardized error handling
    - Uses `ServiceConfigLogic` trait for config loading
    - Uses `ServerConfig` for server configuration
    - Uses `shutdown_signal()` for graceful shutdown

2. **Proto Definitions** (`shared/proto-definitions`)
    - Imports generated gRPC client code
    - Uses `greeter_v1::greeter_client::GreeterClient`
    - Uses request/response types (`HelloRequest`, `HelloResponse`)

3. **Greeter Service** (`services/greeter_service`)
    - Communicates via gRPC
    - Makes `SayHello` and `SayHelloCustom` calls
    - Handles connection pooling and retries

4. **Macros** (`shared/macros`)
    - Uses `#[bridge]` attribute for error conversion
    - Implements `propagate_commonerror!` macro for error bridging

## Key Components

### Configuration (`src/config/`)

- **GatewayConfig**: Top-level configuration combining:
    - Base config (service info, logging)
    - Server config (host, port)
    - Services config (backend service endpoints)

- **ServicesConfig**: Defines connections to backend services
    - URL endpoints
    - Timeout settings
    - Retry policies

### Error Handling (`src/error/`)

- **GatewayError**: Gateway-specific error type
    - Wraps `CommonError` for shared errors
    - Implements `IntoResponse` for HTTP error responses
    - Provides detailed error chains via `report_chain()`

### Services (`src/services/`)

#### Greeter Client (`greeter/`)
- **GreeterServiceClient**: Wrapper around gRPC client
    - Manages persistent channel connections
    - Implements `say_hello()` and `say_hello_custom()`
    - Handles request/response transformation

#### Handlers
- **hello_handler**: HTTP endpoint that calls greeter service
    - Extracts path parameters
    - Makes gRPC calls
    - Adds timestamps
    - Returns JSON responses

#### Root Handler
- **root_handler**: Health check endpoint (/)

## Configuration

The gateway uses a hierarchical configuration system:

### File: `config.yaml`
```yaml
base:
  service:
    name: "gateway"
    version: "0.1.0"
  logging:
    level: "debug"
    format: "pretty"

server:
  host: "0.0.0.0"
  port: 8080

services:
  greeter:
    url: "http://localhost:50051"
    timeout_ms: 5000
```

### Environment Overrides
- `APP_ENV=prod` → loads `config.prod.yaml`
- `APP__SERVER__PORT=9090` → overrides server port
- `GATEWAY__SERVICES__GREETER__URL=...` → service-specific override

## Running the Service

```bash
# Development
cargo run --bin gateway

# With environment override
APP_ENV=prod cargo run --bin gateway

# With port override
APP__SERVER__PORT=9090 cargo run --bin gateway
```

## API Endpoints

### `GET /`
Health check endpoint
- **Response**: HTML status page

### `GET /hello/{name}`
Say hello via greeter service
- **Parameters**: `name` (path parameter)
- **Response**: JSON
  ```json
  {
    "message": "Hello, Alice!",
    "timestamp": 1704067200
  }
  ```

## Error Handling

The gateway provides detailed error responses:

```json
{
  "error": "Main error message",
  "details": [
    "High-level error",
    "Mid-level cause",
    "Root cause"
  ],
  "code": 500
}
```

## Testing

```bash
# Run all tests
cargo test --package gateway

# Run specific test
cargo test --package gateway test_gateway_config_valid_load
```

## Dependencies on Other Workspace Members

- **common**: Configuration, errors, utilities
- **proto_definitions**: gRPC clients and types
- **macros**: Error conversion macros
- **test_helpers**: Test utilities (dev-dependency)

## Production Considerations

1. **Health Checks**: Currently minimal, should add:
    - Database connectivity checks
    - Backend service health checks
    - Resource utilization metrics

2. **Rate Limiting**: Not implemented
    - Add per-client rate limiting
    - Add per-endpoint rate limiting

3. **Authentication**: Not implemented
    - Add JWT validation
    - Add API key validation

4. **Monitoring**: Basic tracing exists
    - Add metrics (requests/sec, latency)
    - Add distributed tracing
    - Add error rate monitoring

5. **Circuit Breakers**: Not implemented
    - Add circuit breakers for backend calls
    - Add fallback mechanisms