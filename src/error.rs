use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Error working with audio file: {0}")]
    AudioFileError(String),

    #[error("Network error while sending to hub: {0}")]
    TransportError(#[from] reqwest::Error),

    #[error("Hub rejected the data. Status: {0}")]
    HubRejected(u16),
}
