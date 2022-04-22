use rustycoat::core::clock::*;
use rustycoat::core::*;
use rustycoat::widgets::leds::*;

fn main() {
    // Create an LED
    let mut led = Led::new();

    // Create a 1Hz clock and wire it up to the LED.
    let mut clock = Clock::new(1);
    clock.output().connect_to(led.input());

    // Create the computer, add components, and start it up.
    let mut c = Computer::new();
    c.add_async(clock);
    c.add_ui(led);

    c.run();
}
