#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}
