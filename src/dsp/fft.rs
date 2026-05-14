use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

pub fn compute_fft(samples: &[i16]) -> Vec<f32> {
    let len = samples.len();

    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .map(|&val| Complex {
            re: val as f32,
            im: 0.0,
        })
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(len);

    fft.process(&mut buffer);

    buffer.iter().take(len / 2).map(|c| c.norm()).collect()
}
