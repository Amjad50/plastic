//! Some common tools used for the emulator UIs to limit FPs

use std::time::{Duration, Instant};

pub struct MovingAverage {
    values: [f64; 100],
    current_index: usize,
    sum: f64,
}

impl Default for MovingAverage {
    fn default() -> Self {
        Self::new()
    }
}

impl MovingAverage {
    pub fn new() -> Self {
        Self {
            values: [0.0; 100],
            current_index: 0,
            sum: 0.0,
        }
    }

    pub fn add(&mut self, value: f64) {
        self.sum -= self.values[self.current_index];
        self.sum += value;
        self.values[self.current_index] = value;
        self.current_index = (self.current_index + 1) % self.values.len();
    }

    pub fn average(&self) -> f64 {
        self.sum / self.values.len() as f64
    }
}

/// Moving average fps counter
pub struct Fps {
    moving_average: MovingAverage,
    last_frame: Instant,
    pub target_fps: f64,
}

impl Fps {
    pub fn new(target_fps: f64) -> Self {
        Self {
            moving_average: MovingAverage::new(),
            last_frame: Instant::now(),
            target_fps,
        }
    }

    // check if we should start a new frame
    // return true if we should start a new frame
    // return false if we should skip this frame
    pub fn start_frame(&mut self) -> bool {
        let duration_per_frame = Duration::from_secs_f64(1.0 / self.target_fps);
        let elapsed = self.last_frame.elapsed();
        if elapsed < duration_per_frame {
            return false;
        }

        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        self.moving_average.add(delta);
        true
    }

    pub fn fps(&self) -> f64 {
        1.0 / self.moving_average.average()
    }

    pub fn remaining(&self) -> Option<Duration> {
        let duration_per_frame = Duration::from_secs_f64(1.0 / self.target_fps);

        let elapsed = self.last_frame.elapsed();

        if elapsed >= duration_per_frame {
            return None;
        }
        let remaining = duration_per_frame - elapsed;
        Some(remaining)
    }
}

/// Process the audio buffer to make it stereo
/// Also add or remove samples to match the current speed
/// more speed_modifier means faster speed and less samples
/// `speed_modifier == 1.0` means normal speed
pub fn process_audio(audio_buffer: &[f32], speed_modifier: f32) -> Vec<f32> {
    let target_len = (audio_buffer.len() as f32 * speed_modifier).ceil() as usize;
    let mut adjusted_buffer = Vec::with_capacity(target_len * 2);

    for i in 0..target_len {
        let src_index_f = i as f32 / speed_modifier;
        let src_index = src_index_f.floor() as usize;
        let next_index = std::cmp::min(src_index + 1, audio_buffer.len() - 1);
        let fraction = src_index_f.fract();

        let sample = if src_index < audio_buffer.len() {
            let current_sample = audio_buffer[src_index];
            let next_sample = audio_buffer[next_index];
            current_sample * (1.0 - fraction) + next_sample * fraction
        } else {
            *audio_buffer.last().unwrap_or(&0.0)
        };
        // Add the sample twice for left and right channels
        adjusted_buffer.push(sample);
        adjusted_buffer.push(sample);
    }

    adjusted_buffer
}
