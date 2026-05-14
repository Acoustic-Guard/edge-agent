use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SpectrumPayloadDto {
    pub device_id: String,
    pub timestamp: u64,
    pub sample_rate: u32,
    pub frequency_spectrum: Vec<f32>,
}
