use std::thread;
use std::time;

fn get_time() -> u64 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap_or_else(|e| e.duration())
        .as_micros() as u64
}

fn delay_1_ms() {
    thread::sleep(time::Duration::from_millis(1));
}

fn add_percent(value: u64, percent: u64) -> u64 {
    (value * 100 + value * percent) / 100
}

pub struct FrameLimiter {
    target_fps: u64,
    time_per_frame: u64,
    begin_timestamp: u64,
    last_frame_timestamp: u64,
    average_frametime: u64,
    average_delaytime: u64,

    frame_counter: u64,
    frame_counter_timestamp: u64,
    fps: u64,

    tolerance_percentage: u64,
}

impl FrameLimiter {
    pub fn new(target_fps: u64) -> FrameLimiter {
        let time_per_frame = 1000000 / target_fps;

        FrameLimiter {
            target_fps,
            time_per_frame,
            last_frame_timestamp: 0,
            average_frametime: time_per_frame / 2,
            average_delaytime: 1000,
            frame_counter: 0,
            frame_counter_timestamp: get_time(),
            fps: 0,
            begin_timestamp: 0,
            tolerance_percentage: 5,
        }
    }

    fn time_left_until_deadline(&self, current: u64) -> u64 {
        let target = self.last_frame_timestamp + self.time_per_frame;
        if current > target {
            0
        } else {
            target - current
        }
    }

    #[allow(dead_code)]
    pub fn target_fps(&self) -> u64 {
        self.target_fps
    }

    #[allow(dead_code)]
    pub fn fps(&self) -> u64 {
        self.fps
    }

    pub fn begin(&mut self) -> bool {
        self.begin_timestamp = get_time();
        let time_left = self.time_left_until_deadline(self.begin_timestamp);
        if time_left > add_percent(self.average_frametime, self.tolerance_percentage) {
            delay_1_ms();

            let elapsed = get_time() - self.begin_timestamp;
            self.average_delaytime = (self.average_delaytime + elapsed) / 2;

            if self.average_frametime < self.average_delaytime {
                self.average_frametime = self.average_delaytime;
            }

            false
        } else {
            true
        }
    }

    pub fn end(&mut self) -> Option<u64> {
        let current = get_time();
        let elapsed = current - self.begin_timestamp;
        let elapsed_since_last_fps_update = current - self.frame_counter_timestamp;

        self.average_frametime = if elapsed < self.average_frametime {
            (self.average_frametime * 2 + elapsed) / 3
        } else {
            (self.average_frametime + elapsed * 2) / 3
        };

        if self.average_frametime < 1000 {
            self.average_frametime = 1000;
        }

        if elapsed_since_last_fps_update > add_percent(self.time_per_frame, 20) {
            self.last_frame_timestamp = current;
        } else {
            self.last_frame_timestamp += self.time_per_frame;
        }

        self.frame_counter += 1;
        if elapsed_since_last_fps_update >= 1000 * 1000 {
            self.fps = (self.frame_counter * 1000 * 1000) / elapsed_since_last_fps_update;
            self.frame_counter_timestamp = current;
            self.frame_counter = 0;

            if self.fps.saturating_sub(self.target_fps) > 5 && self.tolerance_percentage > 2 {
                self.tolerance_percentage -= 1;
            }

            Some(self.fps)
        } else {
            None
        }
    }
}
