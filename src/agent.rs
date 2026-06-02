use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::transport::dto::SpectrumPayloadDto;
use crate::transport::rmq::RmqClient;
use crate::{audio, dsp};

pub struct AcousticAgent {
    pub config: AppConfig,
    pub client: RmqClient,
}

impl AcousticAgent {
    pub async fn build(config: AppConfig) -> Result<Self> {
        let mut retries = 5;

        loop {
            match RmqClient::new(&config.amqp_uri, "acoustic.frames").await {
                Ok(client) => {
                    info!("Successfully connected to RMQ inside agent loop!");
                    return Ok(Self { config, client });
                }
                Err(e) => {
                    if retries == 0 {
                        error!("RabbitMQ connection failed permanently: {}", e);
                        return Err(anyhow::anyhow!("RabbitMQ init failed: {}", e));
                    }
                    warn!(
                        "RabbitMQ is not ready yet, waiting 3 seconds... ({} retries left)",
                        retries
                    );
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    retries -= 1;
                }
            }
        }
    }

    pub async fn run(&self) {
        info!("Starting the multi-threaded agent loop...");

        let latest_bg_noise = Arc::new(RwLock::new(-100.0f32));

        let anomaly_config = self.config.clone();
        let anomaly_client = self.client.clone();
        let noise_for_anomaly = Arc::clone(&latest_bg_noise);

        let anomaly_task = tokio::spawn(async move {
            let frame = match audio::file::read_wav_file(&anomaly_config.audio_file_path) {
                Ok(f) => f,
                Err(e) => {
                    error!("Failed to read audio file: {}", e);
                    return;
                }
            };

            let sample_rate = frame.sample_rate;
            let chunk_size = (sample_rate as f32 * 0.1) as usize;
            let window_size = sample_rate as usize;

            let mut offset = 0;
            let mut rolling_window: VecDeque<i16> = VecDeque::with_capacity(window_size);

            let mut interval = tokio::time::interval(Duration::from_millis(1000));

            loop {
                interval.tick().await;

                let end = offset + chunk_size;
                let chunk = if end <= frame.samples.len() {
                    &frame.samples[offset..end]
                } else {
                    &frame.samples[offset..]
                };

                for &sample in chunk {
                    if rolling_window.len() >= window_size {
                        rolling_window.pop_front();
                    }
                    rolling_window.push_back(sample);
                }

                offset += chunk_size;
                if offset >= frame.samples.len() {
                    offset = 0;
                }

                if chunk.is_empty() {
                    continue;
                }

                let peak_db = dsp::features::compute_dbfs(chunk);

                *noise_for_anomaly.write().await = peak_db;

                if peak_db > -30.0 {
                    info!(
                        "Anomaly detected! Peak DB: {:.2}. Calculating FFT...",
                        peak_db
                    );

                    let window_vec: Vec<i16> = rolling_window.iter().copied().collect();
                    let spectrum = dsp::fft::compute_fft(&window_vec);

                    let timestamp_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;

                    let payload = SpectrumPayloadDto {
                        sensor_id: anomaly_config.device_id.clone(),
                        captured_at_ms: timestamp_ms,
                        latitude: anomaly_config.latitude,
                        longitude: anomaly_config.longitude,
                        fft_bins: spectrum.into_iter().take(100).collect(),
                        sample_rate_hz: sample_rate,
                        peak_db,
                    };

                    let routing_key = format!("sensor.anomaly.{}", anomaly_config.device_id);
                    if let Err(e) = anomaly_client.send_message(&routing_key, &payload).await {
                        error!("Failed to send anomaly: {}", e);
                    }
                }
            }
        });

        let telemetry_config = self.config.clone();
        let telemetry_client = self.client.clone();
        let noise_for_telemetry = Arc::clone(&latest_bg_noise);
        let telemetry_interval = self.config.loop_interval_secs;

        let telemetry_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(telemetry_interval));

            loop {
                interval.tick().await;

                let current_noise = *noise_for_telemetry.read().await;

                let timestamp_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let payload = SpectrumPayloadDto {
                    sensor_id: telemetry_config.device_id.clone(),
                    captured_at_ms: timestamp_ms,
                    latitude: telemetry_config.latitude,
                    longitude: telemetry_config.longitude,
                    fft_bins: vec![],
                    sample_rate_hz: 44100,
                    peak_db: current_noise,
                };

                info!("Sending Telemetry. Current Peak DB: {:.2}", current_noise);

                let routing_key = format!("sensor.telemetry.{}", telemetry_config.device_id);
                if let Err(e) = telemetry_client.send_message(&routing_key, &payload).await {
                    error!("Failed to send telemetry: {}", e);
                }
            }
        });

        let _ = tokio::join!(anomaly_task, telemetry_task);
    }
}
