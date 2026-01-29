use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};
use proto_definitions::greeter_v1::greeter_server::Greeter;
use proto_definitions::greeter_v1::{CustomHelloRequest, HelloRequest, HelloResponse};

#[derive(Clone)]
pub struct GreeterClient {
    endpoint: String,
    timeout: Duration,
}

impl GreeterClient {
    pub fn new(endpoint: String, timeout: Option<Duration>) -> GreeterClient {
        let default_timeout = Duration::from_secs(30);

        GreeterClient {
            endpoint,
            timeout: timeout.unwrap_or(default_timeout),
        }
    }

    /// Get current Unix timestamp
    fn get_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64
    }
}