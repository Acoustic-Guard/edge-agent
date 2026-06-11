use crate::audio::frame::AudioFrame;
use crate::error::AgentError;
use hound::WavReader;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::{error, info};

pub fn _read_wav_file(path: &str) -> Result<AudioFrame, AgentError> {
    let mut reader =
        WavReader::open(path).map_err(|e| AgentError::AudioFileError(e.to_string()))?;

    let metadata = reader.spec();
    let sample_rate = metadata.sample_rate;
    let samples: Vec<i16> = reader.samples::<i16>().filter_map(Result::ok).collect();

    Ok(AudioFrame {
        sample_rate,
        samples,
    })
}

pub async fn start_file_stream(path: String, tx: Sender<AudioFrame>) {
    info!("Starting file audio stream from: {}", path);

    let mut reader = match WavReader::open(&path) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to open wav file: {}", e);
            return;
        }
    };

    let sample_rate = reader.spec().sample_rate;
    let samples: Vec<i16> = reader.samples::<i16>().filter_map(Result::ok).collect();

    let chunk_size = sample_rate as usize;
    let mut offset = 0;

    loop {
        let end = std::cmp::min(offset + chunk_size, samples.len());
        let chunk = samples[offset..end].to_vec();

        let frame = AudioFrame {
            sample_rate,
            samples: chunk,
        };

        if tx.send(frame).await.is_err() {
            info!("Audio stream receiver dropped, stopping file stream.");
            break;
        }

        offset += chunk_size;

        if offset >= samples.len() {
            offset = 0;
        }

        sleep(Duration::from_secs(1)).await;
    }
}
