use std::time;

fn get_time() -> u64 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap_or_else(|e| e.duration())
        .as_micros() as u64
}

fn add_percent(value: u64, percent: u64) -> u64 {
    (value * 100 + value * percent) / 100
}

pub struct FrameLimiter {
    target_fps: f64,
    time_per_frame: u64,
    begin_timestamp: u64,
    last_frame_timestamp: u64,
    average_frametime: u64,

    frame_counter: u64,
    frame_counter_timestamp: u64,
    fps: f64,

    time_stamp_a: u64,
    time_stamp_b: u64,

    offset: i64,
}

impl FrameLimiter {
    pub fn new(target_fps: f64) -> FrameLimiter {
        let mut frame_limiter = Self {
            target_fps,
            time_per_frame: (1000_000.0 / target_fps) as u64,
            begin_timestamp: 0,
            last_frame_timestamp: 0,
            average_frametime: 1000,
            frame_counter: 0,
            frame_counter_timestamp: 0,
            fps: 0.,
            offset: 0,
            time_stamp_a: get_time(),
            time_stamp_b: get_time(),
        };

        frame_limiter.average_frametime = frame_limiter.time_per_frame / 2;

        frame_limiter.reset();
        frame_limiter
    }

    #[allow(dead_code)]
    pub fn target_fps(&self) -> f64 {
        self.target_fps
    }

    #[allow(dead_code)]
    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn reset(&mut self) {
        self.frame_counter_timestamp = get_time();
        self.time_stamp_a = get_time();
        self.time_stamp_b = get_time();
        self.begin_timestamp = 0;
        self.last_frame_timestamp = 0;
        self.average_frametime = 1000;
        self.frame_counter = 0;
    }

    pub fn begin(&mut self) {
        self.time_stamp_a = get_time();
        let work_time = self.time_stamp_a - self.time_stamp_b;

        if work_time < self.time_per_frame {
            let sleep_micros = (self.time_per_frame - work_time) as i64 + self.offset;

            if sleep_micros > 0 {
                let delta = std::time::Duration::from_micros(sleep_micros as u64);
                std::thread::sleep(delta);
            }
        }

        self.time_stamp_b = get_time();
    }

    pub fn end(&mut self) -> Option<f64> {
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
            self.fps =
                (self.frame_counter * 1000 * 1000) as f64 / elapsed_since_last_fps_update as f64;
            self.frame_counter_timestamp = current;
            self.frame_counter = 0;

            if self.fps != self.target_fps {
                self.offset -= ((self.target_fps - self.fps) * 100.) as i64;
            }

            Some(self.fps)
        } else {
            None
        }
    }
}
