use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::Receiver;
use tokio::time::Instant;
use tracing::{error, info, warn};

use crate::audio::frame::AudioFrame;
use crate::config::AppConfig;
use crate::dsp;
use crate::telemetry::TelemetryStats;
use crate::transport::dto::AnomalyPayloadDto;
use crate::transport::rmq::RmqClient;

pub struct AcousticAgent {
    pub config: AppConfig,
    pub client: RmqClient,
}

impl AcousticAgent {
    pub async fn build(config: AppConfig) -> Result<Self> {
        let mut retries = config.rmq_max_retries;
        let retry_delay = Duration::from_secs(config.rmq_retry_delay_secs);

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
                        "RabbitMQ is not ready yet, waiting {} seconds... ({} retries left)",
                        config.rmq_retry_delay_secs, retries
                    );
                    tokio::time::sleep(retry_delay).await;
                    retries -= 1;
                }
            }
        }
    }

    pub async fn run(&self, mut rx: Receiver<AudioFrame>) {
        let telemetry_stats = Arc::new(Mutex::new(TelemetryStats::new()));

        let anomaly_config = self.config.clone();
        let anomaly_client = self.client.clone();
        let stats_for_anomaly = Arc::clone(&telemetry_stats);

        let anomaly_task = tokio::spawn(async move {
            let mut last_anomaly_time: Option<Instant> = None;
            let cooldown_duration = Duration::from_secs(anomaly_config.cooldown_duration_secs);

            while let Some(frame) = rx.recv().await {
                let chunk = &frame.samples;
                if chunk.is_empty() {
                    continue;
                }

                let sample_rate = frame.sample_rate;
                let chunk_db = dsp::features::compute_dbfs(chunk);

                let current_avg_db = {
                    let mut stats = stats_for_anomaly.lock().unwrap();
                    stats.add(chunk_db);
                    stats.current_avg()
                };

                if chunk_db > anomaly_config.anomaly_threshold_db {
                    let now = Instant::now();
                    let should_send = match last_anomaly_time {
                        Some(time) => now.duration_since(time) >= cooldown_duration,
                        None => true,
                    };

                    if should_send {
                        info!("Anomaly detected! DB: {:.2}. Preparing payload...", chunk_db);

                        let timestamp_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;

                        let mut payload_fft = Vec::new();
                        let mut payload_raw = Vec::new();

                        if anomaly_config.send_mode == "RAW" {
                            payload_raw = chunk.clone();
                        } else {
                            payload_fft = dsp::fft::compute_fft(chunk)
                                .into_iter()
                                .take(anomaly_config.fft_bins_count)
                                .collect();
                        }

                        let payload = AnomalyPayloadDto {
                            sensor_id: anomaly_config.device_id.clone(),
                            captured_at_ms: timestamp_ms,
                            latitude: anomaly_config.latitude,
                            longitude: anomaly_config.longitude,
                            sample_rate_hz: sample_rate,
                            peak_db: chunk_db,
                            avg_db: current_avg_db,
                            fft_bins: payload_fft,
                            raw_audio: payload_raw,
                        };

                        let routing_key = format!("sensor.anomaly.{}", anomaly_config.device_id);
                        if let Err(e) = anomaly_client.send_message(&routing_key, &payload).await {
                            error!("Failed to send anomaly: {}", e);
                        }

                        last_anomaly_time = Some(now);
                    }
                }
            }
            info!("Audio stream closed, stopping anomaly task.");
        });

        let telemetry_config = self.config.clone();
        let telemetry_client = self.client.clone();
        let stats_for_telemetry = Arc::clone(&telemetry_stats);

        let telemetry_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                telemetry_config.telemetry_interval_secs,
            ));

            loop {
                interval.tick().await;

                let current_stats = {
                    let mut stats = stats_for_telemetry.lock().unwrap();
                    stats.take_and_reset()
                };

                if let Some((avg_db, _max_db)) = current_stats {
                    let timestamp_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;

                    use crate::transport::dto::TelemetryPayloadDto;

                    let payload = TelemetryPayloadDto {
                        sensor_id: telemetry_config.device_id.clone(),
                        captured_at_ms: timestamp_ms,
                        latitude: telemetry_config.latitude,
                        longitude: telemetry_config.longitude,
                        avg_db,
                    };

                    info!("Sending Telemetry. Avg DB: {:.2}", avg_db);

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
