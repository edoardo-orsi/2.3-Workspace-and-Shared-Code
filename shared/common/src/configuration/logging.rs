use crate::configuration::log_format::LogFormat;
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, PartialEq, Eq, Validate, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[validate(custom(function = "validate_log_level", message = "Invalid log level."))]
    pub level: String,

    #[validate(custom(function = "validate_log_format", message = "Invalid log format."))]
    pub format: LogFormat,
}

impl LoggingConfig {
    /// Initialize logging based on configuration
    pub fn init(&self) {
        // Create filter from level
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(self.level.as_str()));

        // Add format layer based on config
        let layer = match self.format {
            LogFormat::Json => fmt::layer()
                .json()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(true)
                .with_span_events(FmtSpan::CLOSE)
                .boxed(),
            LogFormat::Pretty => fmt::layer()
                .pretty()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_span_events(FmtSpan::CLOSE)
                .boxed(),
            LogFormat::Compact => fmt::layer()
                .compact()
                .with_span_events(FmtSpan::CLOSE)
                .boxed(),
        };

        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(layer)
            .try_init();
    }
}

fn validate_log_level(level: &str) -> Result<(), ValidationError> {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if valid_levels.contains(&level) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_log_level"))
    }
}

fn validate_log_format(log_format: &LogFormat) -> Result<(), ValidationError> {
    let valid_formats = ["json", "pretty", "compact"];

    let format = log_format.to_string();
    if valid_formats.contains(&format.to_lowercase().as_str()) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_log_format"))
    }
}
