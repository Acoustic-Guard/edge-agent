use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpectrumPayloadDto {
    pub sensor_id: String,
    pub captured_at_ms: u64,

    pub latitude: f32,
    pub longitude: f32,

    pub fft_bins: Vec<f32>,
    pub sample_rate_hz: u32,

    pub peak_db: f32,
}
