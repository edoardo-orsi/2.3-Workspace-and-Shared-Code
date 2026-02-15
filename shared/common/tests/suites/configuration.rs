#[cfg(test)]
mod tests {
    use common::{BaseConfig, ServiceConfigLogic};
    use serial_test::serial;
    use std::{env, fs};
    use std::path::Path;
    use serde::Deserialize;
    use tempdir::TempDir;
    use test_helpers::create_temp_config;
    use tracing::{debug, error, info, warn};
    use tracing_test::traced_test;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate, PartialEq)]
    struct TestConfig {
        #[validate(length(min = 3))]
        service_name: String,

        #[validate(range(min = 1, max = 65535))]
        port: u16,

        #[serde(default)]
        optional_field: Option<String>,
    }

    impl ServiceConfigLogic for TestConfig {}

    fn create_config_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
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

        create_config_file(
            temp_dir.path(),
            "config.yaml",
            "service_name: 'test-service'\nport: 8080",
        );

        let config = TestConfig::load_and_validate(temp_dir.path()).unwrap();

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.port, 8080);
    }
    #[test]
    #[serial]
    fn test_environment_specific_config() {
        let temp_dir = TempDir::new("test_environment_specific_config").unwrap();

        create_config_file(
            temp_dir.path(),
            "config.yaml",
            "service_name: 'base'\nport: 8080",
        );

        create_config_file(
            temp_dir.path(),
            "config.test.yaml",
            "port: 9090",
        );

        unsafe {
            env::set_var("APP_ENV", "test");
        }

        let config = TestConfig::load_and_validate(temp_dir.path()).unwrap();

        assert_eq!(config.service_name, "base");    // From config.yaml
        assert_eq!(config.port, 9090);              // From config.test.yaml

        unsafe {
            env::remove_var("APP_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_environment_variable_override() {
        let temp_dir = TempDir::new("test_environment_variable_override").unwrap();

        create_config_file(
            temp_dir.path(),
            "config.yaml",
            "service_name: 'base'\nport: 8080",
        );

        // Override with environment variables
        unsafe {
            env::set_var("APP__PORT", "7070");
        }

        let config = TestConfig::load_and_validate(temp_dir.path()).unwrap();

        assert_eq!(config.service_name, "base");    // From config.yaml
        assert_eq!(config.port, 7070);              // From env

        // Cleanup
        unsafe {
            env::remove_var("APP__PORT");
        }
    }

    #[test]
    fn test_validation_failure() {
        let temp_dir = TempDir::new("test_validation_failure").unwrap();

        create_config_file(
            temp_dir.path(),
            "config.yaml",
            "service_name: 'ab'\nport: 8080", // Too short
        );

        let result = TestConfig::load_and_validate(temp_dir.path());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Validation failed"));
    }

    #[test]
    fn test_missing_config_file() {
        let temp_dir = TempDir::new("test_missing_config_file").unwrap();

        let result = TestConfig::load_and_validate(temp_dir.path());

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_load_secrets() {
        let temp_dir = TempDir::new("test_load_secrets").unwrap();
        let secrets_dir = temp_dir.path().join("secrets");
        fs::create_dir(&secrets_dir).unwrap();

        fs::write(secrets_dir.join("api_key"), "secret123\n").unwrap();
        fs::write(secrets_dir.join("password"), "pass456").unwrap();

        let secrets = TestConfig::load_secrets_from_dir(&secrets_dir);

        assert_eq!(secrets.len(), 2);
        assert_eq!(secrets.get("api_key").unwrap(), "secret123");
        assert_eq!(secrets.get("password").unwrap(), "pass456");
    }

    #[test]
    fn test_empty_secrets_directory() {
        let temp_dir = TempDir::new("test_empty_secrets_directory").unwrap();
        let secrets_dir = temp_dir.path().join("secrets");
        fs::create_dir(&secrets_dir).unwrap();

        let secrets = TestConfig::load_secrets_from_dir(&secrets_dir);

        assert_eq!(secrets.len(), 0);
    }

    #[test]
    fn test_secret_with_whitespace() {
        let temp_dir = TempDir::new("test_secret_with_whitespace").unwrap();
        let secrets_dir = temp_dir.path().join("secrets");
        fs::create_dir(&secrets_dir).unwrap();

        fs::write(secrets_dir.join("token"), "  \n  secret  \n  ").unwrap();

        let secrets = TestConfig::load_secrets_from_dir(&secrets_dir);

        assert_eq!(secrets.get("token").unwrap(), "secret");
    }
}
