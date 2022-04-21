use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::core::ports::OutputPin;
use crate::core::AsyncComponent;

pub struct Clock {
    interval: Duration,
    output: OutputPin,
}

impl Clock {
    pub fn new(ticks_per_second: u64) -> Self {
        Self {
            interval: Duration::from_nanos(1_000_000_000 / ticks_per_second / 2),
            output: OutputPin::new(),
        }
    }

    pub fn state(&self) -> bool {
        self.output.value()
    }

    pub fn output(&mut self) -> &mut OutputPin {
        &mut self.output
    }
}

impl AsyncComponent for Clock {
    fn run(&mut self, stop: Arc<AtomicBool>) {
        let start = Instant::now();
        let time;
        let mut next_tick = Instant::now() + self.interval;
        let mut tick_count = 0;
        let mut now;
        loop {
            while {
                now = Instant::now();
                now
            } < next_tick
            {
                thread::sleep(next_tick - now);
            }
            next_tick += self.interval;
            tick_count += 1;
            if stop.load(Ordering::Relaxed) {
                time = start.elapsed();
                break;
            }
            self.output.send(!self.output.value());
        }
        println!(
            "Clock: {} ticks in {} ms, speed {} MHz",
            tick_count,
            time.as_millis(),
            tick_count as f64 / time.as_micros() as f64
        );
    }
}
