use super::dto::SpectrumPayloadDto;
use crate::error::AgentError;
use reqwest::Client;

pub struct HubClient {
    http_client: Client,
    hub_url: String,
}

impl HubClient {
    pub fn new(hub_url: &str) -> Self {
        Self {
            http_client: Client::new(),
            hub_url: hub_url.to_string(),
        }
    }

    pub async fn send_spectrum(&self, payload: &SpectrumPayloadDto) -> Result<(), AgentError> {
        let response = self
            .http_client
            .post(&self.hub_url)
            .json(payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AgentError::HubRejected(response.status().as_u16()))
        }
    }
}
