use std::io::{self, Write};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Clock {
    interval: Duration,
}

impl Clock {
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }

    pub fn start(&mut self) -> JoinHandle<()> {
        let interval = self.interval;
        let mut signal = false;

        thread::spawn(move || loop {
            thread::sleep(interval);
            signal = !signal;
            print!("{}", if signal { "O" } else { "." });
            io::stdout().flush().unwrap();
        })
    }
}
