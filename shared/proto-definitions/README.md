# Proto Definitions

## Overview

The Proto Definitions crate is the **central source of truth** for all gRPC service definitions in your microservices architecture. It automatically discovers, validates, compiles, and generates Rust code from Protocol Buffer files.

## Purpose

This crate provides:
- **Type-safe gRPC interfaces** for both clients and servers
- **Automatic proto compilation** via `build.rs`
- **Version consistency enforcement** between file structure and proto packages
- **Reflection support** for debugging and tooling
- **Serde integration** for JSON interoperability

## Architecture Role

```
┌─────────────────┐
│ .proto files    │
│ (../../protos)  │
└────────┬────────┘
         │
         │ build.rs compiles
         ▼
┌─────────────────┐      ┌──────────────┐
│ proto_defs      │─────>│   Gateway    │
│ (generated Rust)│      └──────────────┘
└────────┬────────┘
         │                ┌──────────────┐
         └───────────────>│   Greeter    │
                          └──────────────┘
```

Both clients (Gateway) and servers (Greeter) import from this single crate, ensuring type compatibility.

## Directory Structure Requirements

The build system **enforces** a strict structure to maintain consistency:

```
protos/                          # Located at ../../protos
├── greeter/
│   └── v1/
│       └── greeter.proto        # package greeter.v1;
├── auth/
│   └── v1/
│       └── auth.proto           # package auth.v1;
└── orders/
    └── v2/
        └── orders.proto         # package orders.v2;
```

### Validation Rules

1. **Version folder required**: All protos must be in a version folder (v1, v2, etc.)
2. **Package must match folder**: If in `v1/` folder, package must end with `.v1`
3. **Compilation fails otherwise**: The build script will abort with a clear error

**Example Error:**
```
error: Proto Version Mismatch Found!
  File:   protos/auth/v2/auth.proto
  Package: auth.v1
  Folder:  v2
Expected folder name to match the last segment of the package name.
```

## How It Works

### Build Process (`build.rs`)

1. **Discovery**: Walks `../../protos` to find all `.proto` files
2. **Parsing**: Extracts `package` declaration from each file
3. **Validation**: Ensures folder name matches package version
4. **Compilation**: Uses `tonic-prost-build` to generate Rust code
5. **Descriptor Sets**: Creates reflection metadata for each service
6. **Code Generation**: Writes `src/generated_protos.rs` with all modules

### Generated Code Structure

For a proto like:
```protobuf
// protos/greeter/v1/greeter.proto
syntax = "proto3";
package greeter.v1;

service Greeter {
  rpc SayHello(HelloRequest) returns (HelloResponse);
}

message HelloRequest {
  string name = 1;
}

message HelloResponse {
  string message = 1;
  int64 timestamp = 2;
}
```

The build generates:
```rust
pub mod greeter_v1 {
    tonic::include_proto!("greeter.v1");
    pub const DESCRIPTOR_SET: &[u8] = 
        tonic::include_file_descriptor_set!("/greeter_v1_descriptor");
}
```

## Usage

### Server Side (Greeter Service)

```rust
use proto_definitions::greeter_v1::{
    greeter_server::{Greeter, GreeterServer},
    HelloRequest, HelloResponse,
};
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct GreeterService;

#[tonic::async_trait]
impl Greeter for GreeterService {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        let name = &request.get_ref().name;
        
        Ok(Response::new(HelloResponse {
            message: format!("Hello, {}!", name),
            timestamp: get_timestamp(),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    
    Server::builder()
        .add_service(GreeterServer::new(GreeterService::default()))
        .serve(addr)
        .await?;
    
    Ok(())
}
```

### Client Side (Gateway)

```rust
use proto_definitions::greeter_v1::{
    greeter_client::GreeterClient,
    HelloRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = GreeterClient::connect("http://localhost:50051").await?;
    
    let request = Request::new(HelloRequest {
        name: "Alice".to_string(),
    });
    
    let response = client.say_hello(request).await?;
    let hello = response.into_inner();
    
    println!("{}", hello.message);
    
    Ok(())
}
```

### Reflection Support

For debugging, transcoding, or tools like `grpcurl`:

```rust
use tonic_reflection::server::Builder;
use proto_definitions::greeter_v1::DESCRIPTOR_SET;

let reflection_service = Builder::configure()
    .register_encoded_file_descriptor_set(DESCRIPTOR_SET)
    .build_v1()?;

Server::builder()
    .add_service(greeter_service)
    .add_service(reflection_service)  // Enable reflection
    .serve(addr)
    .await?;
```

Now you can introspect the service:
```bash
grpcurl -plaintext localhost:50051 list
grpcurl -plaintext localhost:50051 describe greeter.v1.Greeter
```

### JSON Serialization

All generated types include Serde support:

```rust
use proto_definitions::greeter_v1::HelloResponse;
use serde_json;

let response = HelloResponse {
    message: "Hello, World!".to_string(),
    timestamp: 1704067200,
};

// Serialize to JSON
let json = serde_json::to_string(&response)?;
// {"message":"Hello, World!","timestamp":1704067200}

// Deserialize from JSON
let parsed: HelloResponse = serde_json::from_str(&json)?;
```

## Adding a New Proto

### 1. Create the Proto File

```bash
mkdir -p protos/myservice/v1
```

Create `protos/myservice/v1/myservice.proto`:
```protobuf
syntax = "proto3";
package myservice.v1;

service MyService {
  rpc DoSomething(Request) returns (Response);
}

message Request {
  string data = 1;
}

message Response {
  string result = 1;
}
```

### 2. Build

```bash
cargo build --package proto_definitions
```

The build script will:
- Detect the new proto
- Validate the structure (package `myservice.v1` in folder `v1/`)
- Generate `myservice_v1` module
- Update `generated_protos.rs`

### 3. Use in Your Service

```rust
use proto_definitions::myservice_v1::{
    my_service_server::MyService,
    Request, Response,
};
```

## Versioning Strategy

### Why Version Folders?

1. **Breaking Changes**: Deploy v1 and v2 simultaneously
2. **Migration Period**: Clients gradually migrate from v1 → v2
3. **Rollback Safety**: Can revert to v1 if v2 has issues
4. **Clear Evolution**: Version history visible in file structure

### Example Evolution

```
protos/
├── greeter/
│   ├── v1/
│   │   └── greeter.proto    # Original
│   └── v2/
│       └── greeter.proto    # New features, breaking changes
```

Both versions compile to separate modules:
```rust
use proto_definitions::greeter_v1;  // Old clients
use proto_definitions::greeter_v2;  // New clients
```

## Build Configuration

### `build.rs` Features

- **Automatic Discovery**: No manual proto listing
- **Incremental Builds**: Only recompiles changed protos
- **Clear Errors**: Validation failures show exact file/line
- **IDE Support**: Generated code in `src/` for completion
- **Timestamp Tracking**: Comments show last generation time

### Customization

The `tonic-prost-build` configuration:

```rust
tonic_prost_build::configure()
    .file_descriptor_set_path(&descriptor_path)
    .build_server(true)              // Generate server traits
    .build_client(true)              // Generate client code
    .type_attribute(                 // Add Serde to all types
        ".",
        "#[derive(serde::Serialize, serde::Deserialize)]"
    )
    .compile_protos(&[proto], &[&proto_root])?;
```

## Dependencies

### Runtime
- `tonic`: gRPC framework
- `prost`: Protobuf encoding/decoding
- `serde`: JSON serialization

### Build
- `tonic-prost-build`: Proto compilation
- `walkdir`: Recursive file discovery
- `chrono`: Timestamps in generated code

## Testing

The crate is primarily build-time code generation, so testing focuses on:

1. **Compilation**: Does the build succeed?
2. **Usage**: Can services import and use the types?
3. **Reflection**: Do descriptor sets load correctly?

```bash
# Rebuild protos
cargo build --package proto_definitions

# Test dependent services
cargo test --package gateway
cargo test --package greeter_service
```

## Troubleshooting

### "Package version mismatch"

**Problem**: Proto package doesn't match folder name

**Solution**:
```
# Wrong
protos/auth/v1/auth.proto → package auth.v2;

# Right
protos/auth/v1/auth.proto → package auth.v1;
```

### "No proto files found"

**Problem**: Build script can't find `../../protos`

**Solution**: Ensure proto directory exists relative to this crate:
```
workspace/
├── protos/           # Must be here
└── shared/
    └── proto-definitions/
```

### Generated code not updating

**Problem**: IDE shows old types

**Solution**:
```bash
cargo clean --package proto_definitions
cargo build --package proto_definitions
```

## Best Practices

### 1. **Backward Compatibility**
- Add fields, don't remove them
- Use `optional` for new fields
- Deprecate instead of delete

### 2. **Field Numbers**
- Never reuse field numbers
- Reserve deprecated numbers
```protobuf
reserved 2, 15, 9 to 11;
reserved "old_field_name";
```

### 3. **Documentation**
```protobuf
// Service for user authentication and authorization.
service AuthService {
  // Authenticates a user with username and password.
  // Returns JWT token on success.
  rpc Login(LoginRequest) returns (LoginResponse);
}
```

### 4. **Validation**
Use proto validation (not implemented yet):
```protobuf
message CreateUserRequest {
  string email = 1 [(validate.rules).string.email = true];
  string password = 2 [(validate.rules).string.min_len = 8];
}
```

## Integration with Services

### Gateway (Client)
- Imports generated client code
- Creates `GreeterClient<Channel>`
- Makes RPC calls

### Greeter Service (Server)
- Imports generated server trait
- Implements `Greeter` trait
- Uses `GreeterServer::new(impl)`

### Both Share
- Same request/response types
- Same validation rules
- Same serialization format

## Future Enhancements

1. **gRPC-Web**: Add web client support
2. **Validation**: Proto-level validation rules
3. **OpenAPI**: Generate OpenAPI specs from protos
4. **Mock Generation**: Auto-generate test mocks
5. **Breaking Change Detection**: Compare versions automatically