#[cfg(test)]
mod tests {
    use common::{AppError, ServiceConfigLogic};
    use serde::Deserialize;
    use serial_test::serial;
    use std::fs::File;
    use std::io::Write;
    use tempdir::TempDir;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate, PartialEq)]
    struct TestConfig {
        #[validate(length(min = 3))]
        service_name: String,
        port: u16,
    }

    impl ServiceConfigLogic for TestConfig {}

    #[test]
    #[serial]
    fn test_load_config_success() {
        let temp_dir = TempDir::new("test_load_config_success").unwrap();
        let config_path = temp_dir.path();

        // 1. Create base config.yaml
        let base_yaml = "service_name: 'base-service'\nport: 8080";
        let mut base_file = File::create(config_path.join("config.yaml")).unwrap();
        writeln!(base_file, "{}", base_yaml).unwrap();

        // 2. Create env-specific config.dev.yaml (overrides port)
        let dev_yaml = "port: 9090";
        let mut dev_file = File::create(config_path.join("config.dev.yaml")).unwrap();
        writeln!(dev_file, "{}", dev_yaml).unwrap();

        // 3. Set an Env Var override (overrides service_name)
        // Note: In a real CI environment, consider using a lock if tests run in parallel
        unsafe {
            std::env::set_var("APP_ENV", "dev");
            std::env::set_var("APP__SERVICE_NAME", "env-service");
        }

        let result = TestConfig::load_and_validate(config_path);

        // Assertions
        assert!(
            result.is_ok(),
            "Config should load successfully: {:?}",
            result.err()
        );
        let config = result.unwrap();

        assert_eq!(config.service_name, "env-service"); // Overridden by Env
        assert_eq!(config.port, 9090); // Overridden by config.dev.yaml

        // Clean up env to avoid bleeding into other tests
        unsafe {
            std::env::remove_var("APP_ENV");
            std::env::remove_var("APP__SERVICE_NAME");
        }
    }

    #[test]
    #[serial]
    fn test_validation_failure() {
        let temp_dir = TempDir::new("test_load_base_config").unwrap();
        let config_path = temp_dir.path();

        // Create a config that fails the 'min = 3' validation on service_name
        let invalid_yaml = "service_name: 'ab'\nport: 8080";
        let mut file = File::create(config_path.join("config.yaml")).unwrap();
        writeln!(file, "{}", invalid_yaml).unwrap();

        unsafe {
            std::env::set_var("APP_ENV", "test");
        }

        let result = TestConfig::load_and_validate(config_path);

        assert!(result.is_err());
        if let Err(AppError::ConfigError(msg)) = result {
            assert!(msg.contains("Validation failed"));
        } else {
            panic!("Expected ConfigError from validation");
        }

        unsafe {
            std::env::remove_var("APP_ENV");
        }
    }
}
