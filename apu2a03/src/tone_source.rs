use rodio::Source;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub trait APUChannel {
    fn get_output(&mut self) -> f32;
}

pub trait TimedAPUChannel: APUChannel {
    fn timer_clock(&mut self);
}

#[derive(Serialize, Deserialize)]
pub struct BufferedChannel {
    buffer: VecDeque<f32>,
    overusing: bool,
    underusing: bool,
    last: f32,
    recent_record: bool, // did a record happen recently
    recent_output: bool, // did an output request happen recently
                         //
                         // these are used to know if we are now in a bulk recording
                         // stage, which what happens in the APU
}

impl BufferedChannel {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            overusing: false,
            underusing: false,
            last: 0.,
            recent_record: false,
            recent_output: false,
        }
    }

    pub fn get_is_overusing(&self) -> bool {
        self.overusing
    }

    pub fn get_is_underusing(&self) -> bool {
        self.underusing
    }

    pub fn clear_using_flags(&mut self) {
        self.overusing = false;
        self.underusing = false;
    }

    pub fn recored_sample(&mut self, sample: f32) {
        self.buffer.push_back(sample);
        if self.recent_record {
            // 60 FPS
            if self.buffer.len() > (crate::SAMPLE_RATE / 60) as usize && !self.overusing {
                self.underusing = true;
            }
            self.recent_record = false;
        }
        if self.recent_output {
            self.recent_output = false;
            self.recent_record = true;
        }
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn take_current_buffer(&mut self) -> Vec<f32> {
        self.buffer.drain(..).collect()
    }
}

impl APUChannel for BufferedChannel {
    fn get_output(&mut self) -> f32 {
        self.recent_output = true;

        if self.buffer.is_empty() {
            self.overusing = true;
            self.underusing = false;

            self.last
        } else if self.buffer.len() == 1 {
            self.last = self.buffer.pop_front().unwrap();
            // this should not reach here, or just one time
            // buffer is empty [Problem]
            self.last
        } else {
            self.buffer.pop_front().unwrap()
        }
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
