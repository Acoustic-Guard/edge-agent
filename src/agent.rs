use crate::config::AppConfig;
use crate::transport::dto::SpectrumPayloadDto;
use crate::transport::rmq::RmqClient;
use crate::{audio, dsp};
use anyhow::Result;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

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
                        "RabbitMQ port is not ready yet, waiting 3 seconds... ({} retries left)",
                        retries
                    );
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    retries -= 1;
                }
            }
        }
    }

    pub async fn run(&self) {
        let secs = self.config.loop_interval_secs;
        info!("Starting the main agent loop. Interval: {secs} seconds");

        let frame = match audio::file::read_wav_file("assets/mock_test.wav") {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to read audio file: {}", e);
                return;
            }
        };

        let sample_rate = frame.sample_rate;
        let chunk_size = sample_rate as usize;
        let mut offset = 0;

        let mut interval = tokio::time::interval(Duration::from_secs(secs));

        loop {
            interval.tick().await;

            let end = offset + chunk_size;
            let chunk = if end <= frame.samples.len() {
                &frame.samples[offset..end]
            } else {
                &frame.samples[offset..]
            };

            if let Err(e) = self.process_and_send(chunk, sample_rate).await {
                error!("Error in current iteration: {e}");
            }

            offset += chunk_size;
            if offset >= frame.samples.len() {
                offset = 0;
                info!("Reached the end of the audio file. Looping back...");
            }
        }
    }

    async fn process_and_send(&self, chunk: &[i16], sample_rate: u32) -> Result<()> {
        if chunk.is_empty() {
            return Ok(());
        }

        let peak_db = dsp::features::compute_dbfs(chunk);

        let threshold_db = -30.0;
        let is_anomaly = peak_db > threshold_db;

        let timestamp_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

        let mut payload = SpectrumPayloadDto {
            sensor_id: self.config.device_id.clone(),
            captured_at_ms: timestamp_ms,
            latitude: self.config.latitude,
            longitude: self.config.longitude,
            fft_bins: vec![],
            sample_rate_hz: sample_rate,
            peak_db,
        };

        if is_anomaly {
            info!(
                "Anomaly detected! Peak DB: {:.2}. Calculating FFT...",
                peak_db
            );

            let spectrum = dsp::fft::compute_fft(chunk);
            payload.fft_bins = spectrum.into_iter().take(100).collect();

            let routing_key = format!("sensor.anomaly.{}", self.config.device_id);
            self.client.send_message(&routing_key, &payload).await?;
        } else {
            info!(
                "Background noise. Peak DB: {:.2}. Sending Telemetry.",
                peak_db
            );

            let routing_key = format!("sensor.telemetry.{}", self.config.device_id);
            self.client.send_message(&routing_key, &payload).await?;
        }

        Ok(())
    }
}
