pub fn compute_dbfs(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return -120.0;
    }

    let mut sum_squares = 0.0;
    for &sample in samples {
        let normalized = sample as f32 / i16::MAX as f32;
        sum_squares += normalized * normalized;
    }

    let rms = (sum_squares / samples.len() as f32).sqrt();

    if rms < 1e-9 {
        -120.0
    } else {
        20.0 * rms.log10()
    }
}
