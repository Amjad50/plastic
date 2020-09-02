use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread;
use std::time::Duration;

use crossterm::{
    event::{poll, read, Event as crosstermEvent, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};

pub enum Event<I> {
    Input(I),
    Tick,
}
/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event<KeyEvent>>,
    stopped: Arc<AtomicBool>,
}

impl Events {
    pub fn new(tick_rate: Duration) -> Events {
        let (tx, rx) = mpsc::channel();
        let stopped = Arc::new(AtomicBool::new(false));

        enable_raw_mode().unwrap();

        {
            let tx = tx.clone();
            thread::spawn(move || loop {
                if let Ok(_) = poll(Duration::from_millis(10)) {
                    if let Ok(crosstermEvent::Key(key)) = read() {
                        if let Err(_) = tx.send(Event::Input(key)) {
                            return;
                        }
                    }
                }
            })
        };

        {
            let stopped = stopped.clone();
            thread::spawn(move || loop {
                if !stopped.load(Ordering::Relaxed) {
                    if tx.send(Event::Tick).is_err() {
                        break;
                    }
                }
                thread::sleep(tick_rate);
            })
        };
        Events { rx, stopped }
    }

    pub fn set_stopped_state(&self, state: bool) {
        self.stopped.store(state, Ordering::Relaxed);
    }

    pub fn next(&self) -> Result<Event<KeyEvent>, mpsc::TryRecvError> {
        self.rx.try_recv()
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
    }
}
