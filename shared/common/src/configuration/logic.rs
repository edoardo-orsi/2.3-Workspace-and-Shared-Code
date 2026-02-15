use crate::{CommonError, PathExt};
use figment::Figment;
use figment::providers::{Env, Format, Serialized, Yaml};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};
use validator::Validate;

/// Core trait providing configuration loading and validation logic
///
/// This trait implements the complete configuration lifecycle:
/// 1. Discovery of configuration files
/// 2. Hierarchical merging from multiple sources
/// 3. Environment-based overrides
/// 4. Secret injection from filesystem
/// 5. Validation of final configuration
///
/// # Configuration Priority (Later Overrides Earlier)
///
/// 1. `config.yaml` - Base configuration
/// 2. `config.{APP_ENV}.yaml` - Environment-specific (e.g., config.prod.yaml)
/// 3. `APP__*` - Global environment variables
/// 4. `{SERVICE_NAME}__*` - Service-specific environment variables
/// 5. `/run/secrets/*` - Mounted secret files (Kubernetes/Docker)
///
/// # Example Configuration Flow
///
/// ```yaml
/// # config.yaml (base)
/// database:
///   host: "localhost"
///   port: 5432
///   max_connections: 10
///
/// # config.prod.yaml (production overrides)
/// database:
///   max_connections: 100
///
/// # Environment variables (highest priority)
/// APP__DATABASE__HOST=prod-db.internal
/// ```
///
/// Final result in production:
/// ```yaml
/// database:
///   host: "prod-db.internal"      # From env var
///   port: 5432                     # From base
///   max_connections: 100           # From prod config
/// ```
///
/// # Secrets Management
///
/// Secrets are loaded from `/run/secrets/` (configurable via `APP_SECRETS_DIR`).
/// Each file becomes a configuration value where:
/// - Filename = configuration key (lowercased)
/// - File content = configuration value (trimmed)
///
/// Example:
/// ```text
/// /run/secrets/
///   ├── database_password  → config.database_password = "secret123"
///   └── api_key           → config.api_key = "key-abc-xyz"
/// ```
///
/// # Validation
///
/// After merging all sources, the configuration is validated using
/// the `validator` crate. Failed validation prevents service startup.
///
/// ```rust
/// #[derive(Deserialize, Validate)]
/// struct Config {
///     #[validate(length(min = 1))]
///     name: String,
///
///     #[validate(range(min = 1, max = 65535))]
///     port: u16,
///
///     #[validate(required(code = "missing_secret"))]
///     database_password: Option<String>,
/// }
/// ```
///
/// # Usage Examples
///
/// ## Basic Usage
///
/// ```rust
/// use common::{BaseConfig, ServiceConfigLogic};
///
/// let config = BaseConfig::load_auto()?;
/// config.init_logging()?;
/// ```
///
/// ## Custom Service Configuration
///
/// ```rust
/// #[derive(Deserialize, Validate)]
/// struct MyServiceConfig {
///     #[validate(nested)]
///     base: BaseConfig,
///
///     #[validate(url)]
///     database_url: String,
/// }
///
/// impl ServiceConfigLogic for MyServiceConfig {}
///
/// let config = MyServiceConfig::load_auto()?;
/// ```
///
/// ## Manual Path Specification
///
/// ```rust
/// let config = MyServiceConfig::load_and_validate(Path::new("./config"))?;
/// ```
pub trait ServiceConfigLogic: DeserializeOwned + Validate {
    /// Loads and validates configuration from filesystem and environment
    ///
    /// This is the main entry point for configuration loading. It:
    /// 1. Determines environment (from `APP_ENV` or defaults to "dev")
    /// 2. Loads base configuration file
    /// 3. Merges environment-specific overrides
    /// 4. Applies environment variable overrides
    /// 5. Injects secrets from filesystem
    /// 6. Validates the final configuration
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Directory containing config.yaml and environment-specific configs
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Successfully loaded and validated configuration
    /// * `Err(CommonError::ConfigError)` - File not found, parse error, or validation failure
    /// * `Err(CommonError::MissingSecret)` - Required secret not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::path::Path;
    /// use common::{BaseConfig, ServiceConfigLogic};
    ///
    /// let config = BaseConfig::load_and_validate(Path::new("./config"))?;
    /// ```
    ///
    /// # Environment Variables
    ///
    /// - `APP_ENV` - Determines which config.{env}.yaml to load (default: "dev")
    /// - `APP_SECRETS_DIR` - Directory for secret files (default: "/run/secrets")
    /// - `APP__*` - Global configuration overrides
    /// - `{SERVICE_NAME}__*` - Service-specific overrides
    ///
    /// # File Structure
    ///
    /// ```text
    /// config/
    /// ├── config.yaml          # Base configuration (required)
    /// ├── config.dev.yaml      # Development overrides (optional)
    /// ├── config.staging.yaml  # Staging overrides (optional)
    /// └── config.prod.yaml     # Production overrides (optional)
    /// ```
    fn load_and_validate(base_dir: &Path) -> Result<Self, CommonError> {
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());
        let service_prefix = Self::get_env_prefix();
        let secrets_dir = Self::get_secrets_dir();

        info!(
            env = %env,
            service_prefix = %service_prefix,
            base_dir = %base_dir.display(),
            "Loading configuration"
        );

        let base_config_path = base_dir.join("config.yaml");
        let env_config_path = base_dir.join(format!("config.{}.yaml", env));

        // Verify base config exists
        if !base_config_path.exists() {
            let canonical = base_config_path
                .canonicalize()
                .unwrap_or_else(|_| base_config_path.clone());

            return Err(CommonError::ConfigError(format!(
                "Config file not found at: {}. Current directory: {:?}",
                canonical.display(),
                std::env::current_dir()
            )));
        }

        debug!(
            base_config = %base_config_path.display(),
            env_config = %env_config_path.display(),
            "Loading configuration files"
        );

        // Build configuration hierarchy
        let mut figment = Figment::new()
            .merge(Yaml::file(base_config_path))
            .merge(Yaml::file(env_config_path))
            .merge(Env::prefixed("APP__").split("__"))
            .merge(Env::prefixed(&format!("{}_", service_prefix)).split("__"));

        // Inject secrets if the directory exists
        if secrets_dir.exists() {
            if secrets_dir.has_files() {
                info!(
                    secrets_dir = %secrets_dir.display(),
                    "Loading secrets from directory"
                );

                let secrets = Self::load_secrets_from_dir(&secrets_dir);
                debug!(secret_count = secrets.len(), "Loaded secrets");

                figment = figment.merge(Serialized::defaults(secrets));
            } else {
                warn!(
                    secrets_dir = %secrets_dir.display(),
                    "Secrets directory exists but contains no files"
                );
            }
        } else {
            debug!(
                secrets_dir = %secrets_dir.display(),
                "Secrets directory does not exist (this is OK for development)"
            );
        }

        // Extract configuration
        let config: Self = figment.extract().map_err(|e| {
            CommonError::ConfigError(format!(
                "Failed to parse configuration: {}. \
                    Check YAML syntax and environment variable formats.",
                e
            ))
        })?;

        // Validate configuration
        config.validate().map_err(|e| {
            // Check for specific error types first
            for (field_name, field_errors) in e.field_errors() {
                for error in field_errors {
                    // Priority 1: Specific Secret failures
                    if error.code == "missing_secret" {
                        return CommonError::MissingSecret(field_name.to_string());
                    }

                    // Priority 2: General missing fields
                    if error.code == "required" {
                        return CommonError::ConfigError(format!(
                            "Required field '{}' is missing in configuration",
                            field_name
                        ));
                    }

                    // Priority 3: Specific validation messages
                    if let Some(ref message) = error.message {
                        return CommonError::ConfigError(format!(
                            "Validation failed for '{}': {}",
                            field_name, message
                        ));
                    }
                }
            }

            // Generic validation error
            CommonError::ConfigError(format!("Validation failed: {}", e))
        })?;

        info!("Configuration loaded and validated successfully");
        Ok(config)
    }

    /// Extracts service name from the running binary
    ///
    /// Used for:
    /// - Auto-discovery of config directory
    /// - Service-specific environment variable prefix
    /// - Logging context
    ///
    /// # Returns
    ///
    /// The binary filename without path or extension
    ///
    /// # Examples
    ///
    /// ```text
    /// Binary: /usr/local/bin/gateway    → "gateway"
    /// Binary: ./target/debug/greeter    → "greeter"
    /// Binary: service-engine            → "service-engine"
    /// ```
    fn get_service_name() -> String {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| {
                warn!("Could not determine service name from binary, using 'APP'");
                "APP".to_string()
            })
    }

    /// Formats service name for environment variable prefix
    ///
    /// Converts the service name to uppercase and replaces hyphens with
    /// underscores to create valid environment variable names.
    ///
    /// # Returns
    ///
    /// Uppercase, underscore-separated prefix
    ///
    /// # Examples
    ///
    /// ```text
    /// Service: gateway         → GATEWAY__
    /// Service: greeter-service → GREETER_SERVICE__
    /// Service: api-v2          → API_V2__
    /// ```
    fn get_env_prefix() -> String {
        Self::get_service_name().to_uppercase().replace('-', "_")
    }

    /// Gets the secrets directory path
    ///
    /// Checks `APP_SECRETS_DIR` environment variable, defaulting to `/run/secrets`
    /// which is the standard path for Docker and Kubernetes secret mounts.
    ///
    /// # Returns
    ///
    /// Path to secrets directory
    ///
    /// # Environment Variables
    ///
    /// * `APP_SECRETS_DIR` - Custom secrets directory (optional)
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Default (Kubernetes/Docker standard)
    /// /run/secrets/
    ///
    /// # Custom (for development)
    /// APP_SECRETS_DIR=./secrets cargo run
    /// ```
    fn get_secrets_dir() -> std::path::PathBuf {
        std::env::var("APP_SECRETS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/run/secrets"))
    }

    /// Checks if a directory exists and contains at least one file.
    fn has_files(dir: &Path) -> bool {
        dir.is_dir()
            && fs::read_dir(dir)
                .map(|mut i| i.next().is_some()) // Check if at least one entry exists
                .unwrap_or(false)
    }

    /// Automatically discovers the configuration directory
    ///
    /// This method implements intelligent path discovery:
    /// 1. Checks current working directory for config.yaml
    /// 2. Checks `{cwd}/{service_name}/config.yaml`
    /// 3. Falls back to current working directory
    ///
    /// This allows the service to run from:
    /// - The service directory itself (cargo run)
    /// - The workspace root (cargo run --bin service_name)
    /// - Any directory with proper config structure
    ///
    /// # Returns
    ///
    /// The discovered configuration directory path
    ///
    /// # Examples
    ///
    /// ```text
    /// # Running from workspace root
    /// workspace/
    /// ├── gateway/
    /// │   └── config.yaml     ← Found at workspace/gateway/
    /// └── services/
    ///     └── greeter/
    ///         └── config.yaml ← Found at workspace/services/greeter/
    ///
    /// # Running from service directory
    /// gateway/
    /// └── config.yaml         ← Found at current directory
    /// ```
    fn auto_base_dir() -> std::path::PathBuf {
        let service_name = Self::get_service_name();
        let current_working_directory = std::env::current_dir().unwrap_or_default();

        debug!(
            service_name = %service_name,
            cwd = %current_working_directory.display(),
            "Auto-discovering configuration directory"
        );

        if current_working_directory.join("config.yaml").exists() {
            info!(config_dir = %current_working_directory.display(), "Found config in current directory");
            return current_working_directory;
        }

        // Check service subdirectory
        let service_dir = current_working_directory.join(&service_name);
        if service_dir.join("config.yaml").exists() {
            info!(config_dir = %service_dir.display(), "Found config in service subdirectory");
            return current_working_directory.join(&service_name)
        }

        // Check services/{service_name} (for workspace structure)
        let services_dir = current_working_directory.join("services").join(&service_name);
        if services_dir.join("config.yaml").exists() {
            info!(config_dir = %services_dir.display(), "Found config in services directory");
            return services_dir;
        }

        // Fallback to current directory
        warn!(
            cwd = %current_working_directory.display(),
            "Could not auto-discover config directory, using current directory"
        );
        current_working_directory
    }

    /// Fully automated configuration loading
    ///
    /// Combines path discovery and configuration loading into a single call.
    /// This is the recommended way to load configuration in most cases.
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Successfully loaded and validated configuration
    /// * `Err(CommonError)` - Configuration loading or validation failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use common::{BaseConfig, ServiceConfigLogic};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = BaseConfig::load_auto()?;
    ///     config.init_logging()?;
    ///
    ///     // Service logic...
    ///     Ok(())
    /// }
    /// ```
    fn load_auto() -> Result<Self, CommonError> {
        let base_dir = Self::auto_base_dir();
        Self::load_and_validate(&base_dir)
    }

    /// Loads secrets from filesystem directory
    ///
    /// Each file in the directory becomes a configuration value:
    /// - Key: Lowercase filename
    /// - Value: File contents (trimmed)
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory containing secret files
    ///
    /// # Returns
    ///
    /// HashMap of secret key-value pairs
    ///
    /// # Security Notes
    ///
    /// - File permissions should be 0600 (owner read/write only)
    /// - Directory should be mounted read-only
    /// - Secrets are loaded into memory (consider using Secret<T> types)
    ///
    /// # Examples
    ///
    /// ```text
    /// /run/secrets/
    /// ├── database_password  (contents: "secret123")
    /// ├── api_key           (contents: "key-abc-xyz")
    /// └── jwt_secret        (contents: "jwt-secret-here")
    ///
    /// Result:
    /// {
    ///   "database_password": "secret123",
    ///   "api_key": "key-abc-xyz",
    ///   "jwt_secret": "jwt-secret-here"
    /// }
    /// ```
    fn load_secrets_from_dir(dir: &Path) -> HashMap<String, String> {
        let mut secrets = HashMap::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Only process regular files
                if !path.is_file() {
                    continue;
                }

                if let Some(key) = path.file_name().and_then(|n| n.to_str()) {
                    match fs::read_to_string(&path) {
                        Ok(value) => {
                            let trimmed = value.trim().to_string();

                            // Validate secret is not empty
                            if trimmed.is_empty() {
                                warn!(
                                    secret = %key,
                                    "Secret file is empty, skipping"
                                );
                                continue;
                            }

                            debug!(
                                secret = %key,
                                length = trimmed.len(),
                                "Loaded secret"
                            );

                            secrets.insert(key.to_lowercase(), trimmed);
                        }
                        Err(e) => {
                            warn!(
                                secret = %key,
                                error = %e,
                                "Failed to read secret file"
                            );
                        }
                    }
                }
            }
        }
        
        if secrets.is_empty() {
            debug!("No secrets loaded from directory");
        } else {
            info!(count = secrets.len(), "Loaded secrets");
        }

        secrets
    }
}
