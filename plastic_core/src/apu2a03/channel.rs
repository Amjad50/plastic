use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};

pub trait APUChannel: Serialize + for<'de> Deserialize<'de> {
    fn get_output(&mut self) -> f32;
}

pub trait TimedAPUChannel: APUChannel {
    fn timer_clock(&mut self);
}

#[derive(Serialize, Deserialize)]
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
        self.buffer.push_back(sample);
    }

    pub fn take_buffer(&mut self) -> Vec<f32> {
        self.buffer.drain(..).collect()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "C: APUChannel")]
pub struct Dac<C: APUChannel> {
    capacitor: f32,
    channel: C,
}

impl<C: APUChannel> Dac<C> {
    pub fn new(channel: C) -> Self {
        Self {
            capacitor: 0.,
            channel,
        }
    }

    pub fn dac_output(&mut self) -> f32 {
        let dac_in = self.channel.get_output() / 2.2;
        let dac_out = dac_in - self.capacitor;

        self.capacitor = dac_in - dac_out * 0.996;

        dac_out
    }
}

impl<C: APUChannel> Deref for Dac<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.channel
    }
}

impl<C: APUChannel> DerefMut for Dac<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.channel
    }
}
