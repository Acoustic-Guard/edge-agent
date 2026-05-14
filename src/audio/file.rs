use crate::audio::frame::AudioFrame;
use hound::WavReader;

pub fn read_wav_file(path: &str) -> Result<AudioFrame, String> {
    let mut reader = WavReader::open(path).map_err(|e| format!("failed to open WAV file: {e}"))?;

    let metadata = reader.spec();
    let sample_rate = metadata.sample_rate;
    let samples: Vec<i16> = reader.samples::<i16>().filter_map(Result::ok).collect();

    Ok(AudioFrame {
        sample_rate,
        samples,
    })
}
