mod agent;
mod audio;
mod config;
mod domain;
mod dsp;
mod error;
mod transport;
mod telemetry;

use crate::agent::AcousticAgent;
use crate::config::{AppConfig, AudioSourceMode};
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;

/// Main entry point for the edge agent application.
/// Sets up logging, loads configuration, initializes the audio stream (microphone or file),
/// and starts the main agent loop.
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_current_span(false)
        .with_span_list(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("edge agent initialization");

    let config = AppConfig::load()?;
    let (tx, rx) = mpsc::channel(10); // Channel for transmitting audio chunks

    // Spawn the appropriate audio source task based on configuration
    match &config.audio_source_mode {
        AudioSourceMode::Mic => {
            tokio::spawn(async move {
                audio::capture::start_mic_stream(tx).await;
            });
        }
        AudioSourceMode::File(path) => {
            let file_path = path.clone();
            tokio::spawn(async move {
                audio::file::start_file_stream(file_path, tx).await;
            });
        }
    }

    // Build the agent instance with the configured transport and processor
    let agent = AcousticAgent::build(config).await?;
    // Run the main agent processing loop listening to the audio channel
    agent.run(rx).await;

    Ok(())
}
