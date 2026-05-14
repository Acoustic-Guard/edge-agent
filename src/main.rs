mod agent;
mod audio;
mod config;
mod domain;
mod dsp;
mod error;
mod transport;

use crate::agent::AcousticAgent;
use crate::config::AppConfig;
use anyhow::Result;
use tracing::info;

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
    let agent = AcousticAgent::new(config);
    agent.run().await;

    Ok(())
}
