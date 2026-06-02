use anyhow::{Context, Result};
use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub device_id: String,
    pub loop_interval_secs: u64,
    pub amqp_uri: String,
    pub latitude: f32,
    pub longitude: f32,
    pub audio_file_path: String,
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

        let latitude = env::var("LATITUDE")
            .unwrap_or_else(|_| "50.4501".to_string())
            .parse::<f32>()
            .context("LATITUDE must be a valid f32")?;

        let longitude = env::var("LONGITUDE")
            .unwrap_or_else(|_| "30.5234".to_string())
            .parse::<f32>()
            .context("LONGITUDE must be a valid f32")?;

        let audio_file_path =
            env::var("AUDIO_FILE_PATH").unwrap_or_else(|_| "assets/mock_test.wav".to_string());

        Ok(Self {
            device_id,
            loop_interval_secs,
            amqp_uri,
            latitude,
            longitude,
            audio_file_path,
        })
    }
}
