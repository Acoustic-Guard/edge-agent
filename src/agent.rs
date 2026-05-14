use crate::config::AppConfig;
use crate::transport::dto::SpectrumPayloadDto;
use crate::transport::http::HubClient;
use crate::{audio, dsp};
use anyhow::Result;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

pub struct AcousticAgent {
    pub config: AppConfig,
    pub client: HubClient,
}

impl AcousticAgent {
    pub fn new(config: AppConfig) -> Self {
        let client = HubClient::new(&config.hub_url);
        Self { config, client }
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
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let payload = SpectrumPayloadDto {
            device_id: self.config.device_id.clone(),
            timestamp,
            sample_rate: frame.sample_rate,
            frequency_spectrum: compressed_spectrum,
        };

        info!("sending to a hub...");

        match self.client.send_spectrum(&payload).await {
            Ok(_) => info!("data was successfully delivered"),
            Err(e) => error!("error: {e}"),
        }

        Ok(())
    }
}
