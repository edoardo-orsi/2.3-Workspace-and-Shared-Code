#[cfg(test)]
mod tests {
    use std::{env, fs};
    use serial_test::serial;
    use tempdir::TempDir;
    use tonic::codegen::http::StatusCode;
    use tracing::{debug, error, info, warn};
    use tracing_test::traced_test;
    use common::{AppError, Config};

    // Helper to create temporary config file
    fn create_temp_config(dir: &TempDir, name: &str, content: &str) {
        let path = dir.path().join(name);
        fs::write(path, content).unwrap();
    }

    #[traced_test]
    #[test]
    #[serial]
    fn test_logging_levels() {
        // Test different log levels
        debug!("This is a debug message");
        info!("This is an info message");
        warn!("This is a warning");
        error!("This is an error");

        // Verify logs were captured (tracing-test feature)
        assert!(logs_contain("This is an info message"));
    }

    #[traced_test]
    #[test]
    #[serial]
    fn test_structured_fields() {
        // Log with structured fields
        info!(
            user.id = 123,
            user.name = "Alice",
            action = "login",
            "User logged in"
        );

        assert!(logs_contain("user.id"));
        assert!(logs_contain("123"));
        assert!(logs_contain("Alice"));
    }

    #[test]
    #[serial]
    fn test_load_base_config() {
        let temp_dir = TempDir::new("test_load_base_config").unwrap();

        // Create base config
        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
service:
  name: "gateway"
  version: "0.1.0"
logging:
  level: "info"
  format: "pretty"
"#,
        );

        let config = Config::load_from_path(temp_dir.path()).unwrap();

        assert_eq!(config.service.name, "gateway".to_string());
        assert_eq!(config.service.version, "0.1.0".to_string());
        assert_eq!(config.logging.level, "info");
    }
    #[test]
    #[serial]
    fn test_environment_specific_config() {
        let temp_dir = TempDir::new("test_environment_specific_config").unwrap();

        // Create base config
        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
service:
  name: "gateway"
  version: "0.1.0"
logging:
  level: "info"
  format: "pretty"
"#,
        );

        // Create prod config
        create_temp_config(
            &temp_dir,
            "config.prod.yaml",
            r#"
service:
  version: "0.2.0"
logging:
  level: "warn"
  format: "json"
"#,
        );

        // Set environment to prod
        unsafe { env::set_var("APP_ENV", "prod"); }

        let config = Config::load_from_path(temp_dir.path()).unwrap();

        assert_eq!(config.service.version, "0.2.0");            // Overridden
        assert_eq!(config.logging.level, "warn".to_string());   // Overridden
        assert_eq!(config.service.name, "gateway".to_string()); // From base

        unsafe { env::remove_var("APP_ENV"); }
    }

    #[test]
    #[serial]
    fn test_environment_variable_override() {
        let temp_dir = TempDir::new("test_environment_variable_override").unwrap();

        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
service:
  name: "gateway"
  version: "0.1.0"
logging:
  level: "info"
  format: "pretty"
"#,
        );

        // Override with environment variables
        unsafe { env::set_var("APP__SERVICE__NAME", "new_service"); }
        unsafe { env::set_var("APP__LOGGING__LEVEL", "debug"); }

        let config = Config::load_from_path(temp_dir.path()).unwrap();

        assert_eq!(config.service.name, "new_service"); // From env var
        assert_eq!(config.logging.level, "debug"); // From env var

        // Cleanup
        unsafe { env::remove_var("APP__SERVICE__NAME"); }
        unsafe { env::remove_var("APP__LOGGING__LEVEL"); }
    }

    #[test]
    #[serial]
    fn test_validation_empty_service_name() {
        let temp_dir = TempDir::new("test_validation_empty_service_name").unwrap();

        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
service:
  name: ""
  version: "0.1.0"
logging:
  level: "info"
  format: "pretty"
"#,
        );

        let config = Config::load_from_path(temp_dir.path()).unwrap();

        let result = config.validate();
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, AppError::ConfigError(_)));
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);

    }

    #[test]
    #[serial]
    fn test_validation_invalid_log_level() {
        let temp_dir = TempDir::new("test_validation_invalid_log_level").unwrap();

        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
service:
  name: "gateway"
  version: "0.1.0"
logging:
  level: "invalid"
  format: "pretty"
"#,
        );

        let config = Config::load_from_path(temp_dir.path()).unwrap();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid log level"));
    }

    #[test]
    #[serial]
    fn test_validation_success() {
        let temp_dir = TempDir::new("test_validation_success").unwrap();

        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
service:
  name: "gateway"
  version: "0.1.0"
logging:
  level: "info"
  format: "pretty"
"#,
        );

        let config = Config::load_from_path(temp_dir.path()).unwrap();

        assert!(config.validate().is_ok());
    }
}