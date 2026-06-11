use crate::audio::frame::AudioFrame;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use tracing::{error, info};

pub async fn start_mic_stream(tx: Sender<AudioFrame>) {
    info!("Starting microphone stream...");

    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            error!("No default input device found!");
            return;
        }
    };

    let supported_config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to get default input config: {}", e);
            return;
        }
    };

    let sample_format = supported_config.sample_format();

    let stream_config: cpal::StreamConfig = supported_config.into();

    let sample_rate = stream_config.sample_rate;
    let channels = stream_config.channels;
    info!(
        "Mic config: Sample Rate: {}, Channels: {}",
        sample_rate, channels
    );

    let buffer = Arc::new(Mutex::new(Vec::new()));
    let buffer_clone = buffer.clone();

    let err_fn = |err| error!("An error occurred on the input audio stream: {}", err);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            stream_config,
            move |data: &[f32], _: &_| {
                let mut buf = buffer_clone.lock().unwrap();

                for frame in data.chunks(channels as usize) {
                    let sum: f32 = frame.iter().sum();
                    let mono = sum / channels as f32;
                    let sample_i16 = (mono * i16::MAX as f32) as i16;
                    buf.push(sample_i16);
                }

                if buf.len() >= sample_rate as usize {
                    let chunk = buf.clone();
                    buf.clear();

                    let _ = tx.try_send(AudioFrame {
                        sample_rate,
                        samples: chunk,
                    });
                }
            },
            err_fn,
            None,
        ),
        _ => {
            error!("Unsupported sample format. Only F32 is implemented in this example.");
            return;
        }
    }
    .expect("Failed to build input stream");

    stream.play().expect("Failed to play input stream");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}
