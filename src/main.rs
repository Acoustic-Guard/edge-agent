mod agent;
mod audio;
mod config;
mod domain;
mod dsp;
mod error;
mod transport;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
