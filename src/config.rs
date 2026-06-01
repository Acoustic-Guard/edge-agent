use anyhow::{Context, Result};
use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub device_id: String,
    pub loop_interval_secs: u64,
    pub amqp_uri: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        dotenv().ok();

        let device_id = env::var("DEVICE_ID")
            .context("variable DEVICE_ID is not set in .env or environment")?;

        let loop_interval_secs = env::var("LOOP_INTERVAL_SECS")
            .context("variable LOOP_INTERVAL_SECS is not set in .env or environment")?
            .parse::<u64>()
            .context("LOOP_INTERVAL_SECS must be a valid u64 number")?;

        let amqp_uri =
            env::var("AMQP_URI").context("variable AMQP_URI is not set in .env or environment")?;

        Ok(Self {
            device_id,
            loop_interval_secs,
            amqp_uri,
        })
    }
}
