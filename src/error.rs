use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Error working with audio file: {0}")]
    AudioFileError(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}
