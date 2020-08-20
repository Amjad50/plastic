use rodio::Source;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub trait APUChannel {
    fn get_output(&mut self) -> f32;
}

pub trait TimedAPUChannel: APUChannel {
    fn timer_clock(&mut self);
}

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
            if self.buffer.len() > 10 && !self.overusing {
                self.underusing = true;
            }
            self.recent_record = false;
        }
        if self.recent_output {
            self.recent_output = false;
            self.recent_record = true;
        }
    }
}

impl APUChannel for BufferedChannel {
    fn get_output(&mut self) -> f32 {
        self.recent_output = true;

        if self.buffer.len() == 0 {
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

// this is not my own.
// source: https://github.com/koute/pinky/blob/master/nes/src/filter.rs
pub struct Filter {
    delay_00: f32,
    delay_01: f32,
    delay_02: f32,
    delay_03: f32,
    delay_04: f32,
    delay_05: f32,
}

impl Filter {
    pub fn new() -> Filter {
        Filter {
            delay_00: 0.0,
            delay_01: 0.0,
            delay_02: 0.0,
            delay_03: 0.0,
            delay_04: 0.0,
            delay_05: 0.0,
        }
    }

    pub fn apply(&mut self, input: f32) -> f32 {
        let v17 = 0.88915976376199868 * self.delay_05;
        let v14 = -1.8046931203033707 * self.delay_02;
        let v22 = 1.0862126905669063 * self.delay_04;
        let v21 = -2.0 * self.delay_01;
        let v16 = 0.97475300535003617 * self.delay_04;
        let v15 = 0.80752903209625071 * self.delay_03;
        let v23 = 0.022615049608677419 * input;
        let v12 = -1.7848029270188865 * self.delay_00;
        let v04 = -v12 + v23;
        let v07 = v04 - v15;
        let v18 = 0.04410421960695305 * v07;
        let v13 = -1.8500161310426058 * self.delay_01;
        let v05 = -v13 + v18;
        let v08 = v05 - v16;
        let v19 = 1.0876279697671658 * v08;
        let v10 = v19 + v21;
        let v11 = v10 + v22;
        let v06 = v11 - v14;
        let v09 = v06 - v17;
        let v20 = 1.3176796030365203 * v09;
        let output = v20;
        self.delay_05 = self.delay_02;
        self.delay_04 = self.delay_01;
        self.delay_03 = self.delay_00;
        self.delay_02 = v09;
        self.delay_01 = v08;
        self.delay_00 = v07;

        output
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
