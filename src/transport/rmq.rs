use crate::error::AgentError;
use crate::transport::dto::SpectrumPayloadDto;
use lapin::{
    BasicProperties, Connection, ConnectionProperties, ExchangeKind,
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    types::FieldTable,
};
use tracing::{error, info};

pub struct RmqClient {
    channel: lapin::Channel,
    exchange_name: String,
}

impl RmqClient {
    pub async fn new(amqp_uri: &str, exchange_name: &str) -> Result<Self, AgentError> {
        let options = ConnectionProperties::default();

        let connection = Connection::connect(amqp_uri, options)
            .await
            .map_err(|e| AgentError::TransportError(e.to_string()))?;

        info!("connected to RabbitMQ");

        let channel = connection
            .create_channel()
            .await
            .map_err(|e| AgentError::TransportError(e.to_string()))?;

        channel
            .exchange_declare(
                exchange_name.into(),
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| AgentError::TransportError(e.to_string()))?;

        Ok(Self {
            channel,
            exchange_name: exchange_name.to_string(),
        })
    }

    pub async fn send_message(
        &self,
        routing_key: &str,
        payload: &SpectrumPayloadDto,
    ) -> Result<(), AgentError> {
        let payload_bytes = serde_json::to_vec(payload)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        let confirm = self
            .channel
            .basic_publish(
                self.exchange_name.as_str().into(),
                routing_key.into(),
                BasicPublishOptions::default(),
                &payload_bytes,
                BasicProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2),
            )
            .await
            .map_err(|e| AgentError::TransportError(e.to_string()))?
            .await
            .map_err(|e| AgentError::TransportError(e.to_string()))?;

        if confirm.is_ack() {
            Ok(())
        } else {
            error!("RabbitMQ returned NACK");
            Err(AgentError::TransportError("Message NACKed".into()))
        }
    }
}
