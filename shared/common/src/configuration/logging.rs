use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, fmt};
use crate::configuration::log_format::LogFormat;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
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
            LogFormat::Pretty =>
                fmt::layer()
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