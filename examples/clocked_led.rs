use std::io::stdin;

use rustycoat::core::clock::*;
use rustycoat::core::*;
use rustycoat::widgets::leds::*;

fn main() {
    // Create an LED
    let mut led = Led::new();

    // Create a 1MHz clock and wire it up to the CPU.
    let mut clock = Clock::new(1);
    clock.output().connect_to(led.input());

    // Create a computer, add components, and start it up.
    let mut c = Computer::new();
    c.add_async(clock);
    c.add_sync(led);

    c.run();
}