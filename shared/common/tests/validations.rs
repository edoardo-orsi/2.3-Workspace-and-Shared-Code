#[cfg(test)]
mod tests {
    use common::BaseConfig;
    use serial_test::serial;
    use std::fs;
    use tempdir::TempDir;
    use tonic::codegen::http::StatusCode;

    // Helper to create temporary config file
    fn create_temp_config(dir: &TempDir, name: &str, content: &str) {
        let path = dir.path().join(name);
        fs::write(path, content).unwrap();
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

        let result = BaseConfig::load_from_path(temp_dir.path());
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.contains("Service name cannot be empty"));
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

        let result = BaseConfig::load_from_path(temp_dir.path());
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

        let config = BaseConfig::load_from_path(temp_dir.path());

        assert!(config.is_ok());
    }
}
