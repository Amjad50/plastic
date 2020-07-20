use rodio::{Sample, Source};
use std::sync::{Arc, Mutex};

pub trait APUChannel: Iterator
where
    Self::Item: Sample,
{
    fn sample_rate(&self) -> u32 {
        crate::SAMPLE_RATE
    }
}

pub struct APUChannelPlayer<S>
where
    S: APUChannel,
    S::Item: Sample,
{
    source: Arc<Mutex<S>>,
}

impl<S> APUChannelPlayer<S>
where
    S: APUChannel,
    S::Item: Sample,
{
    pub fn new(source: S) -> Self {
        Self {
            source: Arc::new(Mutex::new(source)),
        }
    }

    pub fn from_clone(source: Arc<Mutex<S>>) -> Self {
        Self { source }
    }

    pub fn clone_source(&self) -> Arc<Mutex<S>> {
        self.source.clone()
    }
}

impl<S> Iterator for APUChannelPlayer<S>
where
    S: APUChannel,
    S::Item: Sample,
{
    type Item = S::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.source.lock().unwrap().next()
    }
}

impl<S> Source for APUChannelPlayer<S>
where
    S: APUChannel,
    S::Item: Sample,
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
