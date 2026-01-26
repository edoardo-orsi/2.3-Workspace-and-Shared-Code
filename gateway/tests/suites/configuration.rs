#[cfg(test)]
mod tests {
    use gateway::GatewayConfig;
    use serial_test::serial;
    use tempdir::TempDir;
    use test_helpers::create_temp_config;

    #[test]
    #[serial]
    fn test_gateway_config_valid_load() {
        let temp_dir = TempDir::new("test_gateway_config_valid_load").unwrap();

        // 1. Create config.yaml
        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
base:
  service:
    name: "gateway"
    version: "1.0.0"
  logging:
    level: "info"
    format: "pretty"
server:
  host: "127.0.0.1"
  port: 8080
services:
  greeter:
    url: "http://localhost:50051"
    timeout_ms: 1000
    retries: 5
"#,
        );

        // 2. Create the env-specific file (config.dev.yaml is default)
        create_temp_config(&temp_dir, "config.dev.yaml", "{}");

        // 3. Load
        let result = GatewayConfig::load_from_path(temp_dir.path());

        assert!(
            result.is_ok(),
            "Config should load successfully: {:?}",
            result.err()
        );
        let config = result.unwrap();

        assert_eq!(config.base.service.name, "gateway");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.services.greeter.url, "http://localhost:50051");
    }

    #[test]
    #[serial]
    fn test_validation_invalid_url() {
        let temp_dir = TempDir::new("test_validation_invalid_url").unwrap();

        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
base:
  service:
    name: "gateway"
    version: "1.0.0"
  logging: { level: "info", format: "compact" }
server:
  host: "localhost"
  port: 8080
services:
  greeter:
    url: "not-a-url"  # This should fail validation
"#,
        );
        create_temp_config(&temp_dir, "config.dev.yaml", "{}");

        let result = GatewayConfig::load_from_path(temp_dir.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Validation failed"));
        assert!(err_msg.contains("url"));
    }

    #[test]
    #[serial]
    fn test_validation_port_out_of_range() {
        let temp_dir = TempDir::new("test_validation_port_out_of_range").unwrap();

        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
base:
  service: { name: "test", version: "1" }
  logging: { level: "info", format: "json" }
server:
  host: "localhost"
  port: 70000 # Invalid port (> 65535)
services:
  greeter: { url: "http://localhost" }
"#,
        );
        create_temp_config(&temp_dir, "config.dev.yaml", "{}");

        let result = GatewayConfig::load_from_path(temp_dir.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("expected u16"));
    }

    #[test]
    #[serial]
    fn test_validation_port_is_zero() {
        let temp_dir = TempDir::new("test_validation_port_out_of_range").unwrap();

        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
base:
  service: { name: "test", version: "1" }
  logging: { level: "info", format: "json" }
server:
  host: "localhost"
  port: 0 # Invalid port (< 1)
services:
  greeter: { url: "http://localhost" }
"#,
        );
        create_temp_config(&temp_dir, "config.dev.yaml", "{}");

        let result = GatewayConfig::load_from_path(temp_dir.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Port cannot be less than 1 or higher than 65.535"));
    }

    #[test]
    #[serial]
    fn test_env_override() {
        let temp_dir = TempDir::new("test_env_override").unwrap();

        create_temp_config(
            &temp_dir,
            "config.yaml",
            r#"
base:
  service: { name: "base", version: "1" }
  logging: { level: "info", format: "json" }
server: { host: "localhost", port: 8080 }
services: { greeter: { url: "http://localhost" } }
"#,
        );
        // config.dev.yaml overrides the port
        create_temp_config(&temp_dir, "config.dev.yaml", "server: { port: 9999 }");

        let config = GatewayConfig::load_from_path(temp_dir.path()).unwrap();

        assert_eq!(config.server.port, 9999);
    }
}
