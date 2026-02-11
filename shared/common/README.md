# Common Library

## Overview

The Common library is the foundational shared crate providing configuration management, error handling, logging, and utilities used across all services in the microservices architecture.

## Purpose

This library enforces consistency across services by providing:
- **Standardized configuration loading** with environment-specific overrides
- **Unified error handling** with proper error chains and HTTP status codes
- **Structured logging** with multiple output formats
- **Service utilities** for graceful shutdown and server configuration
- **Extension traits** for enhanced functionality

## Architecture Role

```
┌─────────────┐
│   Gateway   │──┐
└─────────────┘  │
                 │
┌─────────────┐  │      ┌────────────┐
│  Greeter    │──┼─────>│   Common   │
└─────────────┘  │      └────────────┘
                 │
┌─────────────┐  │
│  Future     │──┘
│  Services   │
└─────────────┘
```

All services depend on Common for core functionality.

## Key Components

### 1. Configuration System (`src/configuration/`)

#### **BaseConfig**
- Contains `ServiceConfig` (name, version)
- Contains `LoggingConfig` (level, format)
- Used as nested field in service-specific configs

#### **ServiceConfigLogic** Trait
The heart of the configuration system. Provides:

```rust
pub trait ServiceConfigLogic {
    // Load from filesystem + env vars
    fn load_and_validate(base_dir: &Path) -> Result<Self, CommonError>;

    // Automatic path discovery
    fn auto_base_dir() -> PathBuf;

    // Fully automated loading
    fn load_auto() -> Result<Self, CommonError>;

    // Secrets management
    fn load_secrets_from_dir(dir: &Path) -> HashMap<String, String>;
}
```

**Loading Priority** (later sources override earlier):
1. `config.yaml` (base configuration)
2. `config.{APP_ENV}.yaml` (environment-specific, e.g., `config.prod.yaml`)
3. `APP__*` environment variables (global overrides)
4. `{SERVICE_NAME}__*` environment variables (service-specific overrides)
5. `/run/secrets/*` files (Kubernetes/Docker secrets)

#### **ServerConfig**
Standard server configuration used by all services:
- `host: String` - Bind address
- `port: u16` - Listen port (validated 1-65535)

### 2. Error Handling (`src/error/`)

#### **CommonError**
Centralized error enum covering:
- Configuration errors
- Network/transport errors
- gRPC status errors
- IO errors
- Validation errors
- Missing secrets

**Key Features:**
- `#[bridge]` attribute for automatic error conversion
- `status_code()` method for HTTP status mapping
- `report_chain()` via `ErrorLogic` trait for error chains
- Integration with `thiserror` for automatic `Display`/`Error` impl

#### **ErrorLogic** Trait
Provides error chain reporting:
```rust
pub trait ErrorLogic {
    fn report_chain(&self) -> Vec<String>;
}
```

This allows any error to expose its full causation chain for debugging.

### 3. Logging (`src/configuration/logging.rs`)

**LoggingConfig** supports three formats:
- **JSON**: Structured logs for production (Grafana Loki, etc.)
- **Pretty**: Human-readable for development
- **Compact**: Minimal output

Features:
- File/line number tracking
- Thread ID tracking
- Span events (tracing integration)
- Environment-based log level (`RUST_LOG`)

### 4. Service Utilities (`src/service_utils/`)

#### **shutdown_signal()**
Graceful shutdown handler:
- Listens for CTRL+C
- Returns when signal received
- Logs shutdown event
- Used by all services for clean termination

#### **ServerConfig**
Validated server configuration with range checks.

### 5. Extension Traits (`src/extension_traits/`)

#### **PathExt**
Extends `std::path::Path`:
```rust
pub trait PathExt {
    fn has_files(&self) -> bool;
}
```

Used to check if secrets directory contains files before attempting to load them.

## Usage Examples

### Basic Service Configuration

```rust
use common::{BaseConfig, ServiceConfigLogic};

#[derive(Deserialize, Validate)]
struct MyServiceConfig {
    #[validate(nested)]
    base: BaseConfig,

    #[validate(nested)]
    server: ServerConfig,

    // Service-specific config
    database_url: String,
}

impl ServiceConfigLogic for MyServiceConfig {}

fn main() -> Result<(), CommonError> {
    // Automatic loading
    let config = MyServiceConfig::load_auto()?;

    // Initialize logging
    config.base.init_logging()?;

    // Use config...
    Ok(())
}
```

### Error Handling with Bridging

```rust
use common::CommonError;
use macros::ExposeStructure;
use thiserror::Error;

#[derive(Error, Debug, ExposeStructure)]
pub enum MyServiceError {
    #[bridge]
    #[error(transparent)]
    Common(#[from] CommonError),

    #[error("Service-specific error: {0}")]
    ServiceError(String),
}

// Use the macro to bridge errors
common::propagate_commonerror!(MyServiceError, Common);
```

### Structured Logging

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(client), fields(user_id = %user_id))]
async fn process_user(user_id: u64, client: &Client) -> Result<(), Error> {
    info!("Processing user");

    match client.fetch_user(user_id).await {
        Ok(user) => {
            info!(username = %user.name, "User fetched");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch user");
            Err(e)
        }
    }
}
```

## Configuration File Structure

### Base Configuration (`config.yaml`)
```yaml
service:
  name: "my-service"
  version: "1.0.0"

logging:
  level: "info"      # trace, debug, info, warn, error
  format: "json"     # json, pretty, compact
```

### Environment-Specific (`config.prod.yaml`)
```yaml
logging:
  level: "warn"
  format: "json"
```

### Environment Variables
```bash
# Global overrides
APP_ENV=prod
APP__LOGGING__LEVEL=debug

# Service-specific overrides
MY_SERVICE__DATABASE_URL=postgres://...
```

### Kubernetes Secrets
Mount secrets to `/run/secrets/`:
```
/run/secrets/
  ├── database_password
  └── api_key
```

These are automatically loaded as configuration values.

## Validation

The library uses the `validator` crate for configuration validation:

```rust
#[derive(Validate)]
struct Config {
    #[validate(length(min = 1))]
    name: String,

    #[validate(range(min = 1, max = 65535))]
    port: u16,

    #[validate(url)]
    endpoint: String,

    #[validate(nested)]
    sub_config: SubConfig,
}
```

Validation errors are converted to `CommonError::ConfigError`.

## Testing

The library includes comprehensive tests:

- **Configuration loading**: Base, environment-specific, env var overrides
- **Validation**: Invalid values, missing fields, type mismatches
- **Logging**: Level filtering, structured fields
- **Error chains**: Multi-level error reporting

```bash
# Run all common tests
cargo test --package common

# Run with output
cargo test --package common -- --nocapture
```

## Dependencies

### Production
- `figment`: Configuration loading with priority merging
- `serde`: Serialization/deserialization
- `validator`: Configuration validation
- `tracing`: Structured logging
- `tracing-subscriber`: Log formatting
- `thiserror`: Error type derivation
- `tonic`: gRPC support
- `tokio`: Async runtime

### Development
- `tracing-test`: Log assertion in tests
- `tempdir`: Temporary directories for config tests
- `serial_test`: Sequential test execution (env vars)
- `test_helpers`: Shared test utilities

## Used By

- **gateway**: HTTP → gRPC gateway service
- **greeter_service**: Example gRPC service
- **Future services**: All new services should use this library

## Design Principles

1. **Convention over Configuration**: Automatic path discovery, sensible defaults
2. **Environment Parity**: Same config structure in dev/staging/prod
3. **Fail Fast**: Validation at startup, not runtime
4. **Observable**: Structured logging with context
5. **12-Factor App**: Environment-based configuration
6. **Kubernetes Native**: Secrets from filesystem mounts

## Future Enhancements

1. **Metrics**: Add Prometheus metrics support
2. **Distributed Tracing**: OpenTelemetry integration
3. **Config Hot Reload**: Watch config files for changes
4. **Typed Secrets**: Enum for secret types
5. **Config Diffing**: Show what changed between environments