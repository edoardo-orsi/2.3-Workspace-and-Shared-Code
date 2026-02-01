use crate::GatewayError;
use proto_definitions::greeter_v1::greeter_client::GreeterClient;
use proto_definitions::greeter_v1::{CustomHelloRequest, HelloRequest, HelloResponse};
use std::time::Duration;
use tonic::transport::Channel;
use tonic::Request;
use tracing::{debug, info, instrument};

#[derive(Debug, Clone)]
pub struct GreeterServiceClient {
    client: GreeterClient<Channel>,
}

impl GreeterServiceClient {
    pub async fn new(endpoint: String, timeout: Option<Duration>) -> Result<Self, GatewayError> {
        let default_timeout = Duration::from_secs(30);

        // Create a persistent channel
        let channel = Channel::from_shared(endpoint)?
            .timeout(timeout.unwrap_or_else(|| default_timeout))
            .connect()
            .await?;

        Ok(GreeterServiceClient {
            client: GreeterClient::new(channel),
        })
    }

    #[instrument(skip(self), err)]
    // err automatically log the error message and set the span status to "Error"
    pub async fn say_hello(&self, name: String) -> Result<HelloResponse, GatewayError> {
        info!(name = %name, "Calling Greeter SayHello");

        let mut client = self.client.clone();

        let request = Request::new(HelloRequest { name });
        let response = client.say_hello(request).await?;
        let inner = response.into_inner();

        debug!(message = %inner.message, "Received response from greeter");
        Ok(inner)
    }

    #[instrument(skip(self), err)]
    // err automatically log the error message and set the span status to "Error"
    pub async fn say_hello_custom(
        &self,
        name: String,
        greeting: String,
    ) -> Result<HelloResponse, GatewayError> {
        info!(name = %name, greeting = %greeting, "Calling Greeter SayHelloCustom");

        let mut client = self.client.clone();

        let request = Request::new(CustomHelloRequest { name, greeting });
        let response = client.say_hello_custom(request).await?;

        let inner = response.into_inner();
        debug!(message = %inner.message, "Received response from greeter");

        Ok(inner)
    }
}
