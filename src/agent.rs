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
        info!("Staring the main agent loop. Interval: {secs} seconds");

        loop {
            if let Err(e) = self.process_and_send().await {
                error!("Error in current iteration: {e}");
                warn!("Waiting for {secs} seconds before the next attempt");
            }

            tokio::time::sleep(Duration::from_secs(secs)).await;
        }
    }

    async fn process_and_send(&self) -> Result<()> {
        info!("reading audio file...");
        let frame = audio::file::read_wav_file("assets/satie.wav")?;

        info!("FFT processing...");
        let spectrum = dsp::fft::compute_fft(&frame.samples);
        let compressed_spectrum: Vec<f32> = spectrum.into_iter().take(100).collect();

        let timestamp_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

        let payload = SpectrumPayloadDto {
            sensor_id: self.config.device_id.clone(),
            captured_at_ms: timestamp_ms,
            latitude: 50.4501,
            longitude: 30.5234,
            fft_bins: compressed_spectrum,
            sample_rate_hz: frame.sample_rate,
            peak_db: -12.5,
        };

        info!("sending to RabbitMQ...");

        match self.client.send_spectrum(&payload).await {
            Ok(_) => info!("data was successfully delivered to RMQ"),
            Err(e) => error!("error sending to RMQ: {e}"),
        }

        Ok(())
    }
}
