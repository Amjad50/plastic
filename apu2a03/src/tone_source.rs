use rodio::Source;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub trait APUChannel {
    fn get_output(&mut self) -> f32;
    fn timer_clock(&mut self);
}

pub struct BufferedChannel {
    buffer: VecDeque<f32>,
}

impl BufferedChannel {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn recored_sample(&mut self, sample: f32) {
        self.buffer.push_back(sample);
    }
}

impl APUChannel for BufferedChannel {
    fn get_output(&mut self) -> f32 {
        if self.buffer.len() <= 1 {
            *self.buffer.front().unwrap_or(&0.)
        } else {
            self.buffer.pop_front().unwrap()
        }
    }

    fn timer_clock(&mut self) {
        unreachable!();
    }
}

pub struct APUChannelPlayer<S>
where
    S: APUChannel,
{
    source: Arc<Mutex<S>>,
}

impl<S> APUChannelPlayer<S>
where
    S: APUChannel,
{
    pub fn from_clone(source: Arc<Mutex<S>>) -> Self {
        Self { source }
    }
}

impl<S> Iterator for APUChannelPlayer<S>
where
    S: APUChannel,
{
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.source.lock().unwrap().get_output())
    }
}

impl<S> Source for APUChannelPlayer<S>
where
    S: APUChannel,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        1
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        crate::SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}
