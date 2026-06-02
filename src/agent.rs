use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::Instant;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::transport::dto::SpectrumPayloadDto;
use crate::transport::rmq::RmqClient;
use crate::{audio, dsp};

#[derive(Debug)]
struct TelemetryStats {
    sum_db: f32,
    max_db: f32,
    count: u32,
}

impl TelemetryStats {
    fn new() -> Self {
        Self {
            sum_db: 0.0,
            max_db: f32::MIN,
            count: 0,
        }
    }

    fn add(&mut self, db: f32) {
        self.sum_db += db;
        self.count += 1;
        if db > self.max_db {
            self.max_db = db;
        }
    }

    fn take_and_reset(&mut self) -> Option<(f32, f32)> {
        if self.count == 0 {
            return None;
        }
        let avg = self.sum_db / self.count as f32;
        let max = self.max_db;

        self.sum_db = 0.0;
        self.count = 0;
        self.max_db = f32::MIN;

        Some((avg, max))
    }
}

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
        let telemetry_stats = Arc::new(Mutex::new(TelemetryStats::new()));

        let anomaly_config = self.config.clone();
        let anomaly_client = self.client.clone();
        let stats_for_anomaly = Arc::clone(&telemetry_stats);

        let anomaly_task = tokio::spawn(async move {
            let frame = match audio::file::read_wav_file(&anomaly_config.audio_file_path) {
                Ok(f) => f,
                Err(e) => {
                    error!("Failed to read audio file: {}", e);
                    return;
                }
            };

            let sample_rate = frame.sample_rate;
            let chunk_size = sample_rate as usize;
            let mut offset = 0;

            let mut interval = tokio::time::interval(Duration::from_millis(3000));
            let mut last_anomaly_time: Option<Instant> = None;
            let cooldown_duration = Duration::from_millis(3000);

            loop {
                interval.tick().await;

                let end = offset + chunk_size;
                let chunk = if end <= frame.samples.len() {
                    &frame.samples[offset..end]
                } else {
                    &frame.samples[offset..]
                };

                offset += chunk_size;
                if offset >= frame.samples.len() {
                    offset = 0;
                }

                if chunk.is_empty() {
                    continue;
                }

                let peak_db = dsp::features::compute_dbfs(chunk);

                {
                    let mut stats = stats_for_anomaly.lock().unwrap();
                    stats.add(peak_db);
                }

                if peak_db > -30.0 {
                    let now = Instant::now();
                    let should_send = match last_anomaly_time {
                        Some(time) => now.duration_since(time) >= cooldown_duration,
                        None => true,
                    };

                    if should_send {
                        info!(
                            "Anomaly detected! Peak DB: {:.2}. Calculating FFT...",
                            peak_db
                        );

                        let spectrum = dsp::fft::compute_fft(chunk);
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

                        last_anomaly_time = Some(now);
                    }
                }
            }
        });

        let telemetry_config = self.config.clone();
        let telemetry_client = self.client.clone();
        let stats_for_telemetry = Arc::clone(&telemetry_stats);

        let telemetry_interval = /*300*/ 20;

        let telemetry_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(telemetry_interval));

            loop {
                interval.tick().await;

                let current_stats = {
                    let mut stats = stats_for_telemetry.lock().unwrap();
                    stats.take_and_reset()
                };

                if let Some((avg_db, max_db)) = current_stats {
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
                        peak_db: avg_db,
                    };

                    info!(
                        "Sending 5-Min Telemetry. Avg DB: {:.2}, Max DB: {:.2}",
                        avg_db, max_db
                    );

                    let routing_key = format!("sensor.telemetry.{}", telemetry_config.device_id);
                    if let Err(e) = telemetry_client.send_message(&routing_key, &payload).await {
                        error!("Failed to send telemetry: {}", e);
                    }
                }
            }
        });

        let _ = tokio::join!(anomaly_task, telemetry_task);
    }
}
