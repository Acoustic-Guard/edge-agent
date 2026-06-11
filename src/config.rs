use anyhow::{Context, Result};
use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub device_id: String,
    pub amqp_uri: String,
    pub latitude: f32,
    pub longitude: f32,
    pub audio_source_mode: AudioSourceMode,

    pub anomaly_threshold_db: f32,
    pub cooldown_duration_secs: u64,
    pub telemetry_interval_secs: u64,
    pub fft_bins_count: usize,
    pub default_sample_rate: u32,
    pub rmq_max_retries: u32,
    pub rmq_retry_delay_secs: u64,
}

#[derive(Debug, Clone)]
pub enum AudioSourceMode {
    Mic,
    File(String),
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        dotenv().ok();

        let device_id = env::var("DEVICE_ID")
            .context("variable DEVICE_ID is not set in .env or environment")?;

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

        let audio_source = env::var("AUDIO_SOURCE")
            .context("variable AUDIO_SOURCE is not set in .env or environment")?;

        let audio_source_mode = match audio_source.as_str() {
            "MIC" => AudioSourceMode::Mic,
            "FILE" => {
                let audio_file_path = env::var("AUDIO_FILE_PATH")
                    .unwrap_or_else(|_| "assets/collection/nature-001.wav".to_string());
                AudioSourceMode::File(audio_file_path)
            }
            other => anyhow::bail!("Invalid AUDIO_SOURCE '{}'. Expected MIC or FILE", other),
        };

        // Зчитування нових параметрів
        let anomaly_threshold_db = env::var("ANOMALY_THRESHOLD_DB")
            .unwrap_or_else(|_| "-30.0".to_string())
            .parse::<f32>()
            .context("ANOMALY_THRESHOLD_DB must be a valid f32")?;

        let cooldown_duration_secs = env::var("COOLDOWN_DURATION_SECS")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u64>()
            .context("COOLDOWN_DURATION_SECS must be a valid u64")?;

        let telemetry_interval_secs = env::var("TELEMETRY_INTERVAL_SECS")
            .unwrap_or_else(|_| "20".to_string())
            .parse::<u64>()
            .context("TELEMETRY_INTERVAL_SECS must be a valid u64")?;

        let fft_bins_count = env::var("FFT_BINS_COUNT")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<usize>()
            .context("FFT_BINS_COUNT must be a valid usize")?;

        let default_sample_rate = env::var("DEFAULT_SAMPLE_RATE")
            .unwrap_or_else(|_| "44100".to_string())
            .parse::<u32>()
            .context("DEFAULT_SAMPLE_RATE must be a valid u32")?;

        let rmq_max_retries = env::var("RMQ_MAX_RETRIES")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u32>()
            .context("RMQ_MAX_RETRIES must be a valid u32")?;

        let rmq_retry_delay_secs = env::var("RMQ_RETRY_DELAY_SECS")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u64>()
            .context("RMQ_RETRY_DELAY_SECS must be a valid u64")?;

        Ok(Self {
            device_id,
            amqp_uri,
            latitude,
            longitude,
            audio_source_mode,
            anomaly_threshold_db,
            cooldown_duration_secs,
            telemetry_interval_secs,
            fft_bins_count,
            default_sample_rate,
            rmq_max_retries,
            rmq_retry_delay_secs,
        })
    }
}
